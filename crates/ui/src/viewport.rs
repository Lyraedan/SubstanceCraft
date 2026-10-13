//! The 3D viewport panel: renders the scene offscreen with the wgpu renderer, shows the result as an
//! egui image, maps pointer input onto the camera, and projects paint strokes onto the mesh.

use substancecraft_engine::session::{Session, SharedSession, lock};
use substancecraft_geom::glam::Vec2;
use substancecraft_paint::ToolKind;
use substancecraft_render::ViewportRenderer;

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
    painting: bool,
}

impl Viewport {
    /// Draw the viewport and handle input. Returns a paint action for the shell to dispatch.
    pub fn show(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, session: &SharedSession, tool: ViewportTool) -> Option<ViewportAction> {
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

        let action = self.handle_input(ui, &response, rect, session, tool);

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
                    (image, set.materials.clone())
                })
            };
            if let Some((image, materials)) = composite
                && let Some(renderer) = self.renderer.as_mut()
            {
                renderer.set_base_color_texture(device, queue, &image, &materials);
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
