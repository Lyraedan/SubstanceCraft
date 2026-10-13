//! The dockable panel bodies: right dock (texture set list, layers, settings, properties), the
//! bottom assets shelf and the status bar.
//!
//! Mirrors Substance 3D Painter's right column and bottom shelf. Views read tokens and dispatch
//! commands; they never mutate the document directly.

use std::collections::HashMap;

use serde_json::{Value, json};
use substancecraft_bake::BakeMap;
use substancecraft_doc::{BlendMode, Channel, Filter, Generator, LayerKind, Material, Shader, TextureSet, UvTile};
use substancecraft_engine::session::lock;
use substancecraft_paint::ToolKind;

use crate::app::StudioApp;
use crate::icons;
use crate::state::RightTab;
use crate::theme::Tokens;

/// The assets shelf's library selector.
const ASSET_LIBRARIES: &[&str] = &["All Libraries", "Project"];

/// The assets shelf's filter tabs as `(label, icon, category)`.
const ASSET_FILTERS: &[(&str, &str, &str)] = &[
    ("All", "layers", "all"),
    ("Materials", "material", "material"),
    ("Brushes", "brush", "brush"),
    ("Generators", "generator", "generator"),
    ("Filters", "filter", "filter"),
    ("Bake", "assets", "bake"),
    ("Post", "effect", "post"),
];

/// An asset tile's thumbnail.
const TILE: f32 = 76.0;

/// The cache budget used for the status bar's percentage (Painter-style disk-usage readout).
const CACHE_BUDGET_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// How to paint an asset tile's thumbnail.
enum Thumb {
    /// A procedural generator preview, keyed by generator id.
    Generator(&'static str),
    /// A flat colour swatch.
    Swatch([f32; 4]),
    /// A soft brush dab in this colour.
    BrushDab([f32; 4]),
    /// A black-to-white gradient (a filter sample).
    Gradient,
    /// A centred icon, keyed by name.
    Icon(&'static str),
}

/// One clickable asset in the shelf grid.
struct AssetTile {
    label: String,
    tooltip: String,
    action: Option<(&'static str, Value)>,
    thumb: Thumb,
}

/// Paint an asset thumbnail into `rect`.
fn paint_thumb(ui: &egui::Ui, rect: egui::Rect, thumb: &Thumb, thumbs: &HashMap<&'static str, egui::TextureHandle>) {
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(3), Tokens::DARK.window_bg);
    match thumb {
        Thumb::Generator(id) => {
            if let Some(handle) = thumbs.get(id) {
                painter.image(handle.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
        }
        Thumb::Swatch(color) => {
            painter.rect_filled(rect.shrink(6.0), egui::CornerRadius::same(3), linear_color(*color));
        }
        Thumb::BrushDab(color) => {
            painter.circle_filled(rect.center(), rect.width() * 0.38, linear_color(*color));
        }
        Thumb::Gradient => {
            let steps = 16;
            let step_width = rect.width() / steps as f32;
            for step in 0..steps {
                let t = step as f32 / (steps - 1) as f32;
                let x = rect.left() + step_width * step as f32;
                let strip = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(step_width + 1.0, rect.height()));
                painter.rect_filled(strip, egui::CornerRadius::ZERO, egui::Color32::from_gray((t * 255.0) as u8));
            }
        }
        Thumb::Icon(name) => {
            let inner = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width() * 0.55, rect.height() * 0.55));
            crate::icons::image(name, inner.width()).paint_at(ui, inner);
        }
    }
    painter.rect_stroke(rect, egui::CornerRadius::same(3), egui::Stroke::new(1.0, Tokens::DARK.border), egui::StrokeKind::Inside);
}

/// Convert a linear RGBA colour to an opaque display colour.
fn linear_color(color: [f32; 4]) -> egui::Color32 {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    egui::Color32::from_rgb(channel(color[0]), channel(color[1]), channel(color[2]))
}

/// A tile caption, truncated so it never overflows the tile.
fn tile_caption(label: &str) -> String {
    const MAX: usize = 12;
    if label.chars().count() <= MAX {
        label.to_string()
    } else {
        let mut caption: String = label.chars().take(MAX - 1).collect();
        caption.push('…');
        caption
    }
}

/// Collect the effect ids used by a layer stack (recursively), for the `Project` library filter.
fn collect_effect_ids(stack: &substancecraft_doc::LayerStack, out: &mut Vec<&'static str>) {
    for layer in &stack.layers {
        if let Some(effect) = layer.effect_kind() {
            out.push(effect.id());
        }
        if let Some(children) = layer.child_stack() {
            collect_effect_ids(children, out);
        }
    }
}

/// A flattened view of one layer for the UI.
struct LayerView {
    name: String,
    type_id: &'static str,
    visible: bool,
    opacity: f32,
    blend: BlendMode,
    mask: Option<f32>,
    color: Option<[f32; 4]>,
    value: Option<f32>,
}

/// Build a UI view of a layer.
fn layer_view(layer: &substancecraft_doc::Layer) -> LayerView {
    let (color, value) = match &layer.kind {
        LayerKind::Fill { color, value } => (Some(*color), Some(*value)),
        _ => (None, None),
    };
    LayerView {
        name: layer.name.clone(),
        type_id: layer.kind.type_id(),
        visible: layer.visible,
        opacity: layer.opacity,
        blend: layer.blend,
        mask: layer.mask.map(|mask| mask.coverage()),
        color,
        value,
    }
}

/// A flattened view of one texture set for the list tree.
struct SetRow {
    name: String,
    resolution: u32,
    shader: &'static str,
    triangles: usize,
    tiles: Vec<UvTile>,
}

impl StudioApp {
    /// The right dock's top panel: the texture set tree.
    pub(crate) fn texture_set_list_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.strong("Texture Set List");
        ui.separator();

        let (sets, active, isolate) = {
            let session = lock(&self.session);
            let project = session.project();
            let sets: Vec<SetRow> = project
                .texture_sets
                .iter()
                .enumerate()
                .map(|(index, set)| SetRow {
                    name: set.name.clone(),
                    resolution: set.resolution,
                    shader: set.shader.label(),
                    triangles: project.triangles_for_set(index),
                    tiles: set.uv_tiles.clone(),
                })
                .collect();
            (sets, project.active_texture_set, session.settings().isolate_active_set)
        };

        if sets.is_empty() {
            ui.label(egui::RichText::new("No project loaded").color(Tokens::DARK.text_dim));
            return;
        }

        let mut actions: Vec<(&'static str, Value)> = Vec::new();
        egui::ScrollArea::vertical().max_height(240.0).id_salt("sc.sets.tree").show(ui, |ui| {
            for (index, row) in sets.iter().enumerate() {
                let selected = index == active;
                ui.horizontal(|ui| {
                    if ui.selectable_label(selected, &row.name).clicked() {
                        actions.push(("texture_set.select", json!({ "index": index })));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{}²", row.resolution)).color(Tokens::DARK.text_dim));
                        ui.label(egui::RichText::new(row.shader).color(Tokens::DARK.text_dim));
                    });
                });
                if selected {
                    ui.indent(("sc.sets.tiles", index), |ui| {
                        ui.label(
                            egui::RichText::new(format!("UV Tiles ({})  ·  {} tris", row.tiles.len(), row.triangles)).color(Tokens::DARK.text_dim),
                        );
                        for tile in &row.tiles {
                            ui.horizontal(|ui| {
                                ui.label(tile.udim().to_string());
                                ui.label(egui::RichText::new("No name").color(Tokens::DARK.text_dim));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format!("{}²", row.resolution)).color(Tokens::DARK.text_dim));
                                });
                            });
                        }
                    });
                }
            }
        });

        ui.separator();
        let mut solo = isolate;
        if ui.checkbox(&mut solo, "Solo active set").changed() {
            actions.push(("texture_set.set_solo", json!({ "enabled": solo })));
        }

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The right dock's `Texture Set Settings` tab: identity, resolution, channels, tiles, shader.
    pub(crate) fn texture_set_settings_panel(&mut self, ui: &mut egui::Ui) {
        let (active, name, resolution, channels, tiles, shader, params) = {
            let session = lock(&self.session);
            let project = session.project();
            let Some(set) = project.active_set() else { return };
            (project.active_texture_set, set.name.clone(), set.resolution, set.channels.clone(), set.uv_tiles.clone(), set.shader, set.shader_params)
        };

        let mut actions: Vec<(&'static str, Value)> = Vec::new();

        ui.add_space(2.0);
        if self.rename_index != active {
            self.rename_index = active;
            self.rename_buffer = name;
        }
        ui.horizontal(|ui| {
            ui.label("Name");
            let response = ui.add(egui::TextEdit::singleline(&mut self.rename_buffer).desired_width(f32::INFINITY));
            if response.lost_focus() {
                let trimmed = self.rename_buffer.trim().to_string();
                if !trimmed.is_empty() {
                    actions.push(("texture_set.rename", json!({ "name": trimmed })));
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Res");
            for value in [512u32, 1024, 2048, 4096] {
                if ui.selectable_label(resolution == value, value.to_string()).clicked() {
                    actions.push(("texture_set.set_resolution", json!({ "resolution": value })));
                }
            }
        });

        ui.add_space(4.0);
        ui.label("Channels");
        for channel in Channel::ALL {
            let mut enabled = channels.contains(&channel);
            if ui.checkbox(&mut enabled, channel.label()).changed() {
                actions.push(("texture_set.set_channel", json!({ "channel": channel.id(), "enabled": enabled })));
            }
        }

        ui.add_space(4.0);
        ui.label("UV Tiles");
        ui.horizontal_wrapped(|ui| {
            for tile in &tiles {
                if ui.small_button(format!("{} ×", tile.udim())).clicked() {
                    actions.push(("texture_set.remove_uv_tile", json!({ "udim": tile.udim() })));
                }
            }
            if ui.small_button("+ Tile").clicked() {
                let next = tiles.iter().map(|tile| tile.udim()).max().unwrap_or(1001).saturating_add(1);
                actions.push(("texture_set.add_uv_tile", json!({ "udim": next })));
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Shader");
            egui::ComboBox::from_id_salt("sc.set.shader").selected_text(shader.label()).show_ui(ui, |ui| {
                for option in Shader::ALL {
                    if ui.selectable_label(option == shader, option.label()).clicked() {
                        actions.push(("texture_set.set_shader", json!({ "shader": option.id() })));
                    }
                }
            });
        });
        let mut metallic = params.metallic;
        if ui.add(egui::Slider::new(&mut metallic, 0.0..=1.0).text("Metallic")).changed() {
            actions.push(("shader.set", json!({ "metallic": metallic })));
        }
        let mut roughness = params.roughness;
        if ui.add(egui::Slider::new(&mut roughness, 0.0..=1.0).text("Roughness")).changed() {
            actions.push(("shader.set", json!({ "roughness": roughness })));
        }
        let mut specular = params.specular;
        if ui.add(egui::Slider::new(&mut specular, 0.0..=1.0).text("Specular")).changed() {
            actions.push(("shader.set", json!({ "specular": specular })));
        }
        let mut tint = params.base_color_tint;
        ui.horizontal(|ui| {
            ui.label("Tint");
            if ui.color_edit_button_rgb(&mut tint).changed() {
                actions.push(("shader.set", json!({ "tint": tint })));
            }
        });

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The right dock's middle section: a tab strip over Layers and Texture Set Settings.
    pub(crate) fn right_section(&mut self, ui: &mut egui::Ui) {
        let current = lock(&self.session).workspace().right_tab;
        let mut chosen = current;
        ui.horizontal(|ui| {
            for tab in RightTab::ALL {
                if ui.selectable_label(current == tab, tab.label()).clicked() {
                    chosen = tab;
                }
            }
        });
        if chosen != current {
            self.dispatch_and_report("ui.set_right_tab", &json!({ "tab": chosen.id() }));
        }
        ui.separator();
        match chosen {
            RightTab::Layers => self.layer_stack_panel(ui),
            RightTab::Settings => self.texture_set_settings_panel(ui),
        }
    }

    /// The layer stack list and the selected layer's controls.
    pub(crate) fn layer_stack_panel(&mut self, ui: &mut egui::Ui) {
        let (views, can_undo, can_redo) = {
            let session = lock(&self.session);
            let views = session.project().active_set().map(|set| set.layers.layers.iter().map(layer_view).collect::<Vec<_>>()).unwrap_or_default();
            (views, session.can_undo(), session.can_redo())
        };

        if views.is_empty() {
            self.active_layer = usize::MAX;
        } else if self.active_layer >= views.len() {
            self.active_layer = views.len() - 1;
        }

        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            if ui.add(egui::Button::image_and_text(icons::image("add", 14.0), "Fill")).clicked() {
                self.report(self.dispatch("layer.add_fill", &Value::Null));
                self.active_layer = usize::MAX;
            }
            if ui.add(egui::Button::image_and_text(icons::image("add", 14.0), "Group")).clicked() {
                self.report(self.dispatch("layer.add_group", &Value::Null));
                self.active_layer = usize::MAX;
            }
            let has_selection = self.active_layer != usize::MAX;
            if ui.add_enabled(has_selection, egui::Button::image(icons::image("delete", 14.0))).on_hover_text("Delete layer").clicked() {
                self.report(self.dispatch("layer.delete", &json!({ "index": self.active_layer })));
            }
            if ui.add_enabled(has_selection, egui::Button::image(icons::image("up", 14.0))).on_hover_text("Move up").clicked() {
                self.report(self.dispatch("layer.move", &json!({ "index": self.active_layer, "direction": "up" })));
                self.active_layer = self.active_layer.saturating_add(1);
            }
            if ui.add_enabled(has_selection, egui::Button::image(icons::image("down", 14.0))).on_hover_text("Move down").clicked() {
                self.report(self.dispatch("layer.move", &json!({ "index": self.active_layer, "direction": "down" })));
                self.active_layer = self.active_layer.saturating_sub(1);
            }
            if ui.add_enabled(has_selection, egui::Button::image(icons::image("group", 14.0))).on_hover_text("Group layer").clicked() {
                self.report(self.dispatch("layer.group", &json!({ "index": self.active_layer })));
            }
            if ui.add_enabled(can_undo, egui::Button::image(icons::image("undo", 14.0))).on_hover_text("Undo").clicked() {
                self.report(self.dispatch("edit.undo", &Value::Null));
            }
            if ui.add_enabled(can_redo, egui::Button::image(icons::image("redo", 14.0))).on_hover_text("Redo").clicked() {
                self.report(self.dispatch("edit.redo", &Value::Null));
            }
        });
        ui.separator();

        let mut actions: Vec<(&'static str, Value)> = Vec::new();
        let mut active_layer = self.active_layer;

        ui.columns(2, |columns| {
            egui::ScrollArea::vertical().id_salt("sc.layers.list").show(&mut columns[0], |ui| {
                for (index, view) in views.iter().enumerate().rev() {
                    ui.horizontal(|ui| {
                        let icon = if view.visible { "visibility" } else { "hidden" };
                        if ui.add(egui::Button::image(icons::image(icon, 13.0)).frame(false)).clicked() {
                            actions.push(("layer.set_visible", json!({ "index": index, "visible": !view.visible })));
                        }
                        let label = format!("{}  [{}]", view.name, view.type_id);
                        if ui.selectable_label(index == active_layer, label).clicked() {
                            active_layer = index;
                            actions.push(("layer.select", json!({ "index": index })));
                        }
                    });
                }
                if views.is_empty() {
                    ui.label(egui::RichText::new("No layers — add a fill layer").color(Tokens::DARK.text_dim));
                }
            });

            let column = &mut columns[1];
            if let Some(view) = views.get(active_layer) {
                let index = active_layer;
                column.horizontal(|ui| {
                    ui.label("Blend");
                    egui::ComboBox::from_id_salt("sc.layer.blend").selected_text(view.blend.label()).show_ui(ui, |ui| {
                        for mode in BlendMode::ALL {
                            if ui.selectable_label(mode == view.blend, mode.label()).clicked() {
                                actions.push(("layer.set_blend", json!({ "index": index, "mode": mode.id() })));
                            }
                        }
                    });
                });
                let mut opacity = view.opacity;
                if column.add(egui::Slider::new(&mut opacity, 0.0..=1.0).text("Opacity")).changed() {
                    actions.push(("layer.set_opacity", json!({ "index": index, "opacity": opacity })));
                }
                if let Some(color) = view.color {
                    let mut rgba = color;
                    column.horizontal(|ui| {
                        ui.label("Color");
                        if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                            actions.push(("layer.set_fill_color", json!({ "index": index, "color": rgba })));
                        }
                    });
                }
                if let Some(value) = view.value {
                    let mut scalar = value;
                    if column.add(egui::Slider::new(&mut scalar, 0.0..=1.0).text("Value")).changed() {
                        actions.push(("layer.set_fill_value", json!({ "index": index, "value": scalar })));
                    }
                }
                column.horizontal(|ui| {
                    let mut masked = view.mask.is_some();
                    if ui.checkbox(&mut masked, "Mask").changed() {
                        let value = if masked { json!(1.0) } else { Value::Null };
                        actions.push(("layer.set_mask", json!({ "index": index, "value": value })));
                    }
                    if let Some(mask) = view.mask {
                        let mut scalar = mask;
                        if ui.add(egui::Slider::new(&mut scalar, 0.0..=1.0)).changed() {
                            actions.push(("layer.set_mask", json!({ "index": index, "value": scalar })));
                        }
                    }
                });
            }
        });

        self.active_layer = active_layer;
        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The right dock's bottom section: Properties — Paint (material + mask previews, brush).
    pub(crate) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Properties");
            ui.separator();
            ui.label(egui::RichText::new("Paint").color(Tokens::DARK.text_dim));
        });
        ui.separator();

        let (revision, set, material, mask) = {
            let session = lock(&self.session);
            let project = session.project();
            let set = project.active_set().cloned();
            let material = project
                .active_set()
                .and_then(|set| set.materials.first())
                .and_then(|id| project.scene.materials.get(*id as usize))
                .cloned()
                .unwrap_or_default();
            let mask = project.active_set().and_then(|set| {
                let layers = &set.layers.layers;
                let index = if self.active_layer < layers.len() { self.active_layer } else { layers.len().saturating_sub(1) };
                layers.get(index).and_then(|layer| layer.mask.map(|mask| mask.coverage()))
            });
            (session.texture_set_revision(), set, material, mask)
        };
        if let Some(set) = &set {
            self.update_layer_preview(ui.ctx(), revision, set, &material);
        }
        let preview = self.layer_preview.as_ref().map(|(_, handle)| handle.clone());

        let (color, size, symmetry) = {
            let session = lock(&self.session);
            let brush = *session.brush();
            (brush.color, brush.screen_radius_px, brush.symmetry)
        };
        let mut actions: Vec<(&'static str, Value)> = Vec::new();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                match &preview {
                    Some(handle) => {
                        ui.add(egui::Image::new(handle).fit_to_exact_size(egui::vec2(72.0, 72.0)));
                    }
                    None => {
                        ui.allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::hover());
                    }
                }
                ui.label(egui::RichText::new("Material").color(Tokens::DARK.text_dim));
            });
            ui.vertical(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::hover());
                let gray = ((mask.unwrap_or(0.0).clamp(0.0, 1.0)) * 255.0) as u8;
                ui.painter().rect_filled(rect, egui::CornerRadius::same(3), egui::Color32::from_gray(gray));
                ui.painter().rect_stroke(rect, egui::CornerRadius::same(3), egui::Stroke::new(1.0, Tokens::DARK.border), egui::StrokeKind::Inside);
                ui.label(egui::RichText::new(if mask.is_some() { "Mask" } else { "No mask" }).color(Tokens::DARK.text_dim));
            });
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Paint Brush").strong());
                ui.horizontal(|ui| {
                    ui.label("Color");
                    let mut rgba = color;
                    if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                        actions.push(("brush.set_color", json!({ "color": rgba })));
                    }
                });
                let mut radius = size;
                if ui.add(egui::Slider::new(&mut radius, 1.0..=256.0).fixed_decimals(0).text("Size")).changed() {
                    actions.push(("brush.set_screen_radius", json!({ "radius": radius })));
                }
                ui.label(egui::RichText::new(format!("Symmetry: {}", symmetry.label())).color(Tokens::DARK.text_dim));
            });
        });

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The bottom assets shelf: a library selector, search, icon filter row and a thumbnail grid of
    /// materials, brushes, generators and filters, with bake and post settings below.
    pub(crate) fn assets_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Assets");
            ui.separator();
            let library = ASSET_LIBRARIES.get(self.asset_library).copied().unwrap_or("All Libraries");
            egui::ComboBox::from_id_salt("sc.assets.library").selected_text(library).show_ui(ui, |ui| {
                for (index, name) in ASSET_LIBRARIES.iter().enumerate() {
                    if ui.selectable_label(self.asset_library == index, *name).clicked() {
                        self.asset_library = index;
                    }
                }
            });
            ui.separator();
            ui.add(icons::image("search", 14.0));
            ui.add(egui::TextEdit::singleline(&mut self.asset_search).desired_width(150.0).hint_text("Search"));
            ui.separator();
            for (index, (label, icon, _)) in ASSET_FILTERS.iter().enumerate() {
                if ui.add(egui::Button::image(icons::image(icon, 16.0)).selected(self.asset_filter == index)).on_hover_text(*label).clicked() {
                    self.asset_filter = index;
                }
            }
        });
        ui.separator();

        let category = ASSET_FILTERS.get(self.asset_filter).map(|(_, _, category)| *category).unwrap_or("all");
        let query = self.asset_search.trim().to_ascii_lowercase();
        let matches = |label: &str| query.is_empty() || label.to_ascii_lowercase().contains(&query);

        let (mat_name, mat_color, brush_color) = {
            let session = lock(&self.session);
            let project = session.project();
            let material = project
                .active_set()
                .and_then(|set| set.materials.first())
                .and_then(|id| project.scene.materials.get(*id as usize))
                .cloned()
                .unwrap_or_default();
            (material.name, material.base_color, session.brush().color)
        };

        let project_ids: Vec<&'static str> = if self.asset_library == 1 {
            let session = lock(&self.session);
            let mut ids = Vec::new();
            if let Some(set) = session.project().active_set() {
                collect_effect_ids(&set.layers, &mut ids);
            }
            ids
        } else {
            Vec::new()
        };

        let show = |category_name: &str| category == "all" || category == category_name;
        let mut tiles: Vec<AssetTile> = Vec::new();

        if show("material") && matches(&mat_name) {
            tiles.push(AssetTile {
                label: mat_name.clone(),
                tooltip: format!("Material · {mat_name} — click to pick its base colour"),
                action: Some(("brush.set_color", json!({ "color": mat_color }))),
                thumb: Thumb::Swatch(mat_color),
            });
        }
        if show("brush") {
            for tool in ToolKind::ALL {
                if matches(tool.label()) {
                    tiles.push(AssetTile {
                        label: tool.label().to_string(),
                        tooltip: format!("{} tool", tool.label()),
                        action: Some(("tool.set", json!({ "tool": tool.id() }))),
                        thumb: Thumb::BrushDab(brush_color),
                    });
                }
            }
        }
        if show("generator") {
            for id in Generator::IDS {
                if self.asset_library == 1 && !project_ids.contains(&id) {
                    continue;
                }
                let label = Generator::from_id(id).map(|generator| generator.label()).unwrap_or(id);
                if matches(label) {
                    tiles.push(AssetTile {
                        label: label.to_string(),
                        tooltip: format!("Add {label} effect layer"),
                        action: Some(("layer.add_generator", json!({ "generator": id }))),
                        thumb: Thumb::Generator(id),
                    });
                }
            }
        }
        if show("filter") {
            for id in Filter::IDS {
                if self.asset_library == 1 && !project_ids.contains(&id) {
                    continue;
                }
                let label = Filter::from_id(id).map(|filter| filter.label()).unwrap_or(id);
                if matches(label) {
                    tiles.push(AssetTile {
                        label: label.to_string(),
                        tooltip: format!("Add {label} filter effect"),
                        action: Some(("layer.add_filter", json!({ "filter": id }))),
                        thumb: Thumb::Gradient,
                    });
                }
            }
        }
        if show("bake") && matches("Bake Maps") {
            tiles.push(AssetTile {
                label: "Bake Maps".to_string(),
                tooltip: "Bake the mesh maps".to_string(),
                action: Some(("bake.run", Value::Null)),
                thumb: Thumb::Icon("assets"),
            });
        }

        self.ensure_generator_thumbs(ui.ctx());

        let (resolution, samples, high_poly, bake_revision, first_bake, log, vignette, bloom) = {
            let session = lock(&self.session);
            let first_bake = session.baked_maps().first().map(|(_, image)| (image.width, image.height, image.to_rgba8_linear()));
            (
                session.bake_settings().resolution,
                session.bake_settings().samples,
                session.high_poly().is_some(),
                session.bake_revision(),
                first_bake,
                session.bake_log().to_vec(),
                session.settings().vignette,
                session.settings().bloom,
            )
        };
        if let Some((width, height, bytes)) = &first_bake
            && self.bake_preview.as_ref().is_none_or(|(revision, _)| *revision != bake_revision)
        {
            let color = egui::ColorImage::from_rgba_unmultiplied([*width as usize, *height as usize], bytes);
            let handle = ui.ctx().load_texture("sc.bake_preview", color, egui::TextureOptions::LINEAR);
            self.bake_preview = Some((bake_revision, handle));
        }
        let bake_preview = self.bake_preview.as_ref().map(|(_, handle)| handle.clone());
        let has_bake = first_bake.is_some();

        let mut actions: Vec<(&'static str, Value)> = Vec::new();
        let thumbs = std::mem::take(&mut self.generator_thumbs);
        egui::ScrollArea::vertical().id_salt("sc.assets").show(ui, |ui| {
            if tiles.is_empty() {
                if !query.is_empty() {
                    ui.label(egui::RichText::new("No matching assets").color(Tokens::DARK.text_dim));
                }
            } else {
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    for tile in &tiles {
                        ui.allocate_ui_with_layout(egui::vec2(TILE, TILE + 18.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                            let (rect, response) = ui.allocate_exact_size(egui::vec2(TILE, TILE), egui::Sense::click());
                            paint_thumb(ui, rect, &tile.thumb, &thumbs);
                            if response.clicked()
                                && let Some(action) = tile.action.clone()
                            {
                                actions.push(action);
                            }
                            ui.label(egui::RichText::new(tile_caption(&tile.label)).small().color(Tokens::DARK.text_dim));
                        })
                        .response
                        .on_hover_text(&tile.tooltip);
                    }
                });
            }

            if query.is_empty() && (category == "all" || category == "bake") {
                ui.add_space(6.0);
                ui.separator();
                ui.strong("Bake Settings");
                ui.horizontal(|ui| {
                    ui.label("Res");
                    for value in [128u32, 256, 512] {
                        if ui.selectable_label(resolution == value, value.to_string()).clicked() {
                            actions.push(("bake.set_resolution", json!({ "resolution": value })));
                        }
                    }
                });
                let mut samples_value = samples;
                if ui.add(egui::Slider::new(&mut samples_value, 1..=64).text("AO Samples")).changed() {
                    actions.push(("bake.set_samples", json!({ "samples": samples_value })));
                }
                ui.horizontal(|ui| {
                    if ui.button("Import High-Poly…").clicked()
                        && let Some(path) = rfd::FileDialog::new().add_filter("3D meshes", &["obj", "gltf", "glb"]).pick_file()
                    {
                        actions.push(("file.import_highpoly", json!({ "path": path.to_string_lossy() })));
                    }
                    if ui.add_enabled(high_poly, egui::Button::new("Clear")).clicked() {
                        actions.push(("bake.clear_highpoly", Value::Null));
                    }
                });
                if let Some(handle) = &bake_preview {
                    ui.add(egui::Image::new(handle).fit_to_exact_size(egui::vec2(96.0, 96.0)));
                }
                if has_bake {
                    for map in BakeMap::ALL {
                        ui.horizontal(|ui| {
                            ui.label(map.label());
                            if ui.small_button("Save…").clicked()
                                && let Some(path) =
                                    rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name(format!("{}.png", map.id())).save_file()
                            {
                                actions.push(("bake.save", json!({ "map": map.id(), "path": path.to_string_lossy() })));
                            }
                        });
                    }
                }
                for line in &log {
                    ui.label(egui::RichText::new(line).color(Tokens::DARK.text_dim));
                }
            }

            if query.is_empty() && (category == "all" || category == "post") {
                ui.add_space(6.0);
                ui.separator();
                ui.strong("Post");
                let mut vignette_value = vignette;
                if ui.add(egui::Slider::new(&mut vignette_value, 0.0..=1.0).text("Vignette")).changed() {
                    actions.push(("viewport.set_vignette", json!({ "value": vignette_value })));
                }
                let mut bloom_value = bloom;
                if ui.add(egui::Slider::new(&mut bloom_value, 0.0..=1.0).text("Bloom")).changed() {
                    actions.push(("viewport.set_bloom", json!({ "value": bloom_value })));
                }
            }
        });
        self.generator_thumbs = thumbs;

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// Generate and cache small previews for the procedural generators (once).
    pub(crate) fn ensure_generator_thumbs(&mut self, ctx: &egui::Context) {
        for id in Generator::IDS {
            if self.generator_thumbs.contains_key(id) {
                continue;
            }
            let Some(generator) = Generator::from_id(id) else { continue };
            let image = substancecraft_material::generate(&generator, 64, 64);
            let color = egui::ColorImage::from_rgba_unmultiplied([image.width as usize, image.height as usize], &image.to_rgba8());
            let handle = ctx.load_texture(format!("sc.gen.{id}"), color, egui::TextureOptions::LINEAR);
            self.generator_thumbs.insert(id, handle);
        }
    }

    /// The bottom status bar: the latest status message, the cache usage and the product version.
    pub(crate) fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&self.status).color(Tokens::DARK.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).color(Tokens::DARK.text_dim));
                ui.separator();
                let bytes = lock(&self.session).cache_bytes();
                let megabytes = bytes as f64 / (1024.0 * 1024.0);
                let percent = bytes.saturating_mul(100) / CACHE_BUDGET_BYTES;
                ui.label(egui::RichText::new(format!("Cache: {megabytes:.1} MB ({percent}%)")).color(Tokens::DARK.text_dim));
            });
        });
    }

    /// Recompute the composited base-colour preview when the texture set revision changes.
    pub(crate) fn update_layer_preview(&mut self, ctx: &egui::Context, revision: u64, set: &TextureSet, material: &Material) {
        if self.layer_preview.as_ref().is_some_and(|(cached, _)| *cached == revision) {
            return;
        }
        let image = substancecraft_compose::evaluate_base_color(set, material, 96, 96);
        let size = [image.width as usize, image.height as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, &image.to_rgba8());
        let handle = ctx.load_texture("sc.layer_preview", color, egui::TextureOptions::LINEAR);
        self.layer_preview = Some((revision, handle));
    }
}
