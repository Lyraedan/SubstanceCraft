//! The 3D viewport panel: renders the scene offscreen with the wgpu renderer, shows the result as an
//! egui image, maps pointer input onto the camera, and projects paint strokes onto the mesh.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use substancecraft_engine::session::{Session, SharedSession, lock};
use substancecraft_geom::glam::Vec2;
use substancecraft_paint::ToolKind;
use substancecraft_render::{ToneMap, ViewportRenderer};

/// The active viewport tool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewportTool {
    /// Paint onto the active texture set (left drag paints).
    #[default]
    Paint,
    /// Orbit the camera (left drag orbits).
    Orbit,
}

impl ViewportTool {
    /// A stable id for state.
    pub fn id(self) -> &'static str {
        match self {
            ViewportTool::Paint => "paint",
            ViewportTool::Orbit => "orbit",
        }
    }
}

/// An interaction the viewport produced for the shell to turn into a command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewportAction {
    /// Begin a paint stroke at UV.
    StrokeBegin {
        /// U coordinate.
        u: f32,
        /// V coordinate.
        v: f32,
        /// Texture texels per screen pixel.
        scale: f32,
    },
    /// Continue a paint stroke at UV.
    StrokeExtend {
        /// U coordinate.
        u: f32,
        /// V coordinate.
        v: f32,
    },
    /// End the current paint stroke.
    StrokeEnd,
    /// Fill the UV island containing a triangle.
    Polyfill {
        /// The hit triangle index.
        triangle: u32,
    },
    /// Select the active texture set from the viewport's material dropdown.
    SelectTextureSet {
        /// The texture set index.
        index: usize,
    },
}

/// The composite resolution uploaded to the viewport.
const COMPOSITE_RESOLUTION: u32 = 512;

/// The viewport panel state.
#[derive(Default)]
pub struct Viewport {
    renderer: Option<ViewportRenderer>,
    texture_id: Option<egui::TextureId>,
    uploaded_size: (u32, u32),
    uploaded_scene_revision: u64,
    uploaded_environment_revision: u64,
    applied_selection: Option<(u64, bool)>,
    applied_base_revision: Option<u64>,
    last_signature: Option<u64>,
    painting: bool,
}

impl Viewport {
    /// Draw the viewport and handle input. Returns a paint action for the shell to dispatch.
    pub fn show(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, session: &SharedSession, tool: ViewportTool) -> Option<ViewportAction> {
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

        let mut action = self.handle_input(ui, &response, rect, session, tool);

        let Some(render_state) = frame.wgpu_render_state() else {
            ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, crate::theme::Tokens::DARK.viewport_bg);
            return action;
        };

        let pixels_per_point = ui.ctx().pixels_per_point();
        let width = (rect.width() * pixels_per_point).round().max(1.0) as u32;
        let height = (rect.height() * pixels_per_point).round().max(1.0) as u32;

        if self.renderer.is_none() {
            self.renderer = Some(ViewportRenderer::new(&render_state.device));
        }

        // Only re-composite and re-render when something that affects the image changed.
        let signature = {
            let session = lock(session);
            scene_signature(&session, width, height)
        };
        let needs_render = self.last_signature != Some(signature) || self.texture_id.is_none();
        if needs_render {
            self.upload_if_changed(&render_state.device, &render_state.queue, session);
            self.apply_state(&render_state.device, &render_state.queue, session);

            let resized = self.renderer.as_ref().is_none_or(|renderer| renderer.size() != (width, height));
            if resized && let Some(renderer) = self.renderer.as_mut() {
                renderer.resize(&render_state.device, width, height);
                if let Some(old) = self.texture_id.take() {
                    render_state.renderer.write().free_texture(&old);
                }
                if let Some(view) = renderer.color_view() {
                    let id = render_state.renderer.write().register_native_texture(&render_state.device, view, eframe::wgpu::FilterMode::Linear);
                    self.texture_id = Some(id);
                    self.uploaded_size = (width, height);
                }
            }

            if let Some(renderer) = self.renderer.as_mut() {
                let session = lock(session);
                renderer.render(&render_state.device, &render_state.queue, session.camera(), session.settings());
            }
            self.last_signature = Some(signature);
        }

        let Some(texture_id) = self.texture_id else { return action };
        ui.painter().image(texture_id, rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        ui.painter().rect_stroke(rect, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, crate::theme::Tokens::DARK.border), egui::StrokeKind::Inside);
        ui.painter().text(
            rect.left_bottom() + egui::vec2(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{}×{}  ·  {}", self.uploaded_size.0, self.uploaded_size.1, tool.id()),
            egui::FontId::monospace(11.0),
            crate::theme::Tokens::DARK.text_dim,
        );

        // The material dropdown (top-right), mirroring Painter's active-material selector.
        let (set_names, active_set) = {
            let session = lock(session);
            let project = session.project();
            (project.texture_sets.iter().map(|set| set.name.clone()).collect::<Vec<_>>(), project.active_texture_set)
        };
        if !set_names.is_empty() {
            let overlay_width = 170.0;
            let overlay_rect = egui::Rect::from_min_size(
                egui::pos2((rect.right() - overlay_width - 8.0).max(rect.left() + 8.0), rect.top() + 8.0),
                egui::vec2(overlay_width, 22.0),
            );
            let mut selected = None;
            ui.scope_builder(egui::UiBuilder::new().max_rect(overlay_rect), |ui| {
                let active_name = set_names.get(active_set).map(String::as_str).unwrap_or("Material");
                egui::ComboBox::from_id_salt("sc.viewport.material").width(overlay_width).selected_text(active_name).show_ui(ui, |ui| {
                    for (index, name) in set_names.iter().enumerate() {
                        if ui.selectable_label(index == active_set, name).clicked() {
                            selected = Some(index);
                        }
                    }
                });
            });
            if let Some(index) = selected {
                action = Some(ViewportAction::SelectTextureSet { index });
            }
        }

        action
    }

    fn handle_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        session: &SharedSession,
        tool: ViewportTool,
    ) -> Option<ViewportAction> {
        let pointer_delta = ui.input(|input| input.pointer.delta());
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        let aspect = (rect.width() / rect.height().max(1.0)).max(1e-3);

        if tool == ViewportTool::Paint {
            if response.is_pointer_button_down_on() {
                if let Some(position) = response.interact_pointer_pos() {
                    let (active_tool, hit) = {
                        let session = lock(session);
                        (session.tool(), pick_hit(&session, position, rect, aspect))
                    };
                    if let Some((hit, scale)) = hit {
                        if active_tool == ToolKind::Polyfill {
                            if !self.painting {
                                self.painting = true;
                                return Some(ViewportAction::Polyfill { triangle: hit.triangle });
                            }
                            return None;
                        }
                        let begin = !self.painting;
                        self.painting = true;
                        return Some(if begin {
                            ViewportAction::StrokeBegin { u: hit.uv.x, v: hit.uv.y, scale }
                        } else {
                            ViewportAction::StrokeExtend { u: hit.uv.x, v: hit.uv.y }
                        });
                    }
                }
                return None;
            } else if self.painting {
                self.painting = false;
                return Some(ViewportAction::StrokeEnd);
            }
            // Fall through to camera controls for secondary/middle drags and scroll.
        }

        let mut session = lock(session);
        if response.dragged_by(egui::PointerButton::Primary) {
            if tool == ViewportTool::Orbit {
                session.camera_mut().orbit(pointer_delta.x, pointer_delta.y);
            }
        } else if response.dragged_by(egui::PointerButton::Secondary) || response.dragged_by(egui::PointerButton::Middle) {
            session.camera_mut().pan(pointer_delta.x, pointer_delta.y, rect.height());
        }
        if response.hovered() && scroll.abs() > 0.0 {
            session.camera_mut().dolly(scroll * 0.0015);
        }
        None
    }

    /// Re-upload geometry and environment when their revisions change.
    fn upload_if_changed(&mut self, device: &eframe::wgpu::Device, queue: &eframe::wgpu::Queue, session: &SharedSession) {
        let guard = lock(session);
        let scene_revision = guard.scene_revision();
        let environment_revision = guard.environment_revision();
        let scene_dirty = scene_revision != self.uploaded_scene_revision;
        let environment_dirty = environment_revision != self.uploaded_environment_revision;
        if !scene_dirty && !environment_dirty {
            return;
        }

        let Some(renderer) = self.renderer.as_mut() else { return };
        if scene_dirty {
            renderer.set_scene(device, queue, guard.scene());
            self.uploaded_scene_revision = scene_revision;
        }
        if environment_dirty {
            match guard.environment() {
                Some(image) => renderer.set_environment(device, queue, image),
                None => renderer.clear_environment(device),
            }
            self.uploaded_environment_revision = environment_revision;
        }
    }

    /// Apply texture-set selection (solo) and the composited base-colour texture.
    fn apply_state(&mut self, device: &eframe::wgpu::Device, queue: &eframe::wgpu::Queue, session: &SharedSession) {
        let revision = lock(session).texture_set_revision();
        let isolate = lock(session).settings().isolate_active_set;

        if self.applied_selection != Some((revision, isolate)) {
            let materials = {
                let guard = lock(session);
                isolate.then(|| guard.active_materials())
            };
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.set_visible_materials(materials);
            }
            self.applied_selection = Some((revision, isolate));
        }

        if self.applied_base_revision != Some(revision) {
            let composite = {
                let guard = lock(session);
                guard.project().active_set().cloned().map(|set| {
                    let material =
                        set.materials.first().and_then(|id| guard.project().scene.materials.get(*id as usize)).cloned().unwrap_or_default();
                    let image = substancecraft_compose::evaluate_base_color(&set, &material, COMPOSITE_RESOLUTION, COMPOSITE_RESOLUTION);
                    let params = set.shader_params;
                    (image, set.materials.clone(), params.base_color_tint, params.metallic, params.roughness)
                })
            };
            if let Some((image, materials, tint, metallic, roughness)) = composite
                && let Some(renderer) = self.renderer.as_mut()
            {
                renderer.set_base_color_texture(device, queue, &image, &materials);
                renderer.set_material_override(queue, Some((materials, tint, metallic, roughness)));
            }
            self.applied_base_revision = Some(revision);
        }
    }
}

/// Cast a picking ray through a screen position, returning the hit and the texels-per-screen-pixel
/// scale for screen-space brush sizing.
fn pick_hit(session: &Session, position: egui::Pos2, rect: egui::Rect, aspect: f32) -> Option<(substancecraft_geom::Hit, f32)> {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let ndc_x = (position.x - rect.left()) / rect.width() * 2.0 - 1.0;
    let ndc_y = 1.0 - (position.y - rect.top()) / rect.height() * 2.0;
    let camera = session.camera();
    let ray = camera.ray(Vec2::new(ndc_x, ndc_y), aspect);
    let mesh = &session.scene().mesh;
    let hit = substancecraft_geom::raycast(mesh, &ray)?;

    let world_per_px = 2.0 * hit.distance * (camera.fov_y * 0.5).tan() / rect.height().max(1.0);
    let uv_per_world = substancecraft_geom::raycast::uv_density(mesh, hit.triangle);
    let texels = session.paint_resolution() as f32;
    let scale = (world_per_px * uv_per_world * texels).max(0.0);
    Some((hit, scale))
}

/// A cheap hash of everything that changes the rendered image, so the viewport can skip work when
/// nothing has changed.
fn scene_signature(session: &Session, width: u32, height: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    session.scene_revision().hash(&mut hasher);
    session.environment_revision().hash(&mut hasher);
    session.texture_set_revision().hash(&mut hasher);
    session.bake_revision().hash(&mut hasher);

    let camera = session.camera();
    camera.distance.to_bits().hash(&mut hasher);
    camera.yaw.to_bits().hash(&mut hasher);
    camera.pitch.to_bits().hash(&mut hasher);
    for axis in camera.target.to_array() {
        axis.to_bits().hash(&mut hasher);
    }
    camera.projection.id().hash(&mut hasher);

    let settings = session.settings();
    settings.display.id().hash(&mut hasher);
    settings.exposure_stops.to_bits().hash(&mut hasher);
    tone_map_code(settings.tone_map).hash(&mut hasher);
    settings.show_grid.hash(&mut hasher);
    settings.show_background.hash(&mut hasher);
    settings.isolate_active_set.hash(&mut hasher);
    settings.vignette.to_bits().hash(&mut hasher);
    settings.bloom.to_bits().hash(&mut hasher);
    hasher.finish()
}

fn tone_map_code(tone_map: ToneMap) -> u8 {
    match tone_map {
        ToneMap::None => 0,
        ToneMap::Reinhard => 1,
        ToneMap::Aces => 2,
        ToneMap::Agx => 3,
        ToneMap::Neutral => 4,
    }
}
