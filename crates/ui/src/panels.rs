//! The dockable panel bodies: right dock (texture set list, layers, settings, properties), the
//! bottom assets shelf and the status bar.
//!
//! Mirrors Substance 3D Painter's right column and bottom shelf. Views read tokens and dispatch
//! commands; they never mutate the document directly.

use std::collections::HashMap;

use serde_json::{Value, json};
use substancecraft_bake::BakeMap;
use substancecraft_doc::{
    BlendMode, Channel, Effect, FillSource, Filter, Generator, LayerKind, Mask, MaskEffect, Material, MeshMapKind, Projection, ProjectionKind,
    Shader, TextureSet, UvTile,
};
use substancecraft_engine::session::lock;
use substancecraft_paint::ToolKind;

use crate::app::StudioApp;
use crate::icons;
use crate::theme::Tokens;

/// The assets shelf's library selector.
const ASSET_LIBRARIES: &[&str] = &["All Libraries", "Project"];

/// The assets shelf's filter tabs as `(label, icon, category)`.
const ASSET_FILTERS: &[(&str, &str, &str)] = &[
    ("All", "layers", "all"),
    ("Materials", "material", "material"),
    ("Smart Materials", "material", "smart_material"),
    ("Smart Masks", "layers", "smart_mask"),
    ("Brushes", "brush", "brush"),
    ("Generators", "generator", "generator"),
    ("Filters", "filter", "filter"),
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
    mask_effects: Vec<MaskEffect>,
    parts: Option<Vec<u32>>,
    channel_opacity: Vec<(Channel, f32)>,
    color: Option<[f32; 4]>,
    value: Option<f32>,
    source: &'static str,
    generator: Option<&'static str>,
    mesh_map: Option<MeshMapKind>,
    strength: f32,
    projection: Projection,
    effect: Option<Effect>,
}

/// Build a UI view of a layer.
fn layer_view(layer: &substancecraft_doc::Layer) -> LayerView {
    let (color, value, source, generator, mesh_map, strength, projection) = match &layer.kind {
        LayerKind::Fill { color, value, source, projection, strength } => {
            let (generator, mesh_map) = match source {
                FillSource::Procedural { generator } => (Some(generator.id()), None),
                FillSource::MeshMap { kind } => (None, Some(*kind)),
                _ => (None, None),
            };
            (Some(*color), Some(*value), source.id(), generator, mesh_map, *strength, *projection)
        }
        _ => (None, None, "uniform", None, None, 1.0, Projection::default()),
    };
    let mask_effects = match &layer.mask {
        Some(Mask::Stack { effects }) => effects.clone(),
        _ => Vec::new(),
    };
    LayerView {
        name: layer.name.clone(),
        type_id: layer.kind.type_id(),
        visible: layer.visible,
        opacity: layer.opacity,
        blend: layer.blend,
        mask: layer.mask.as_ref().map(|mask| mask.coverage()),
        mask_effects,
        parts: layer.parts.clone(),
        channel_opacity: layer.channel_opacity.clone(),
        color,
        value,
        source,
        generator,
        mesh_map,
        strength,
        projection,
        effect: layer.effect_kind(),
    }
}

/// Append a `layer.set_effect` action for a single parameter change.
fn push_effect_param(actions: &mut Vec<(&'static str, Value)>, index: usize, key: &'static str, value: f32) {
    let mut params = serde_json::Map::new();
    params.insert("index".into(), json!(index));
    params.insert(key.into(), json!(value));
    actions.push(("layer.set_effect", Value::Object(params)));
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
    /// The texture set tree.
    pub(crate) fn texture_set_list_panel(&mut self, ui: &mut egui::Ui) {
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

    /// The layer stack list and the selected layer's controls.
    pub(crate) fn layer_stack_panel(&mut self, ui: &mut egui::Ui) {
        let (views, channels, part_count, can_undo, can_redo) = {
            let session = lock(&self.session);
            let set = session.project().active_set();
            let views = set.map(|set| set.layers.layers.iter().map(layer_view).collect::<Vec<_>>()).unwrap_or_default();
            let channels = set.map(|set| set.channels.clone()).unwrap_or_default();
            (views, channels, session.part_count(), session.can_undo(), session.can_redo())
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
        let (undo_depth, redo_depth) = {
            let session = lock(&self.session);
            (session.undo_depth(), session.redo_depth())
        };
        egui::CollapsingHeader::new(format!("History ({undo_depth} undo / {redo_depth} redo)")).default_open(false).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for back in 1..=undo_depth.min(8) {
                    if ui.button(format!("Undo {back}")).clicked() {
                        self.report(self.dispatch("edit.undo_steps", &json!({ "count": back })));
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for forward in 1..=redo_depth.min(8) {
                    if ui.button(format!("Redo {forward}")).clicked() {
                        self.report(self.dispatch("edit.redo_steps", &json!({ "count": forward })));
                    }
                }
            });
        });
        ui.separator();

        let mut actions: Vec<(&'static str, Value)> = Vec::new();
        let mut active_layer = self.active_layer;

        egui::ScrollArea::vertical().id_salt("sc.layers").show(ui, |ui| {
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

            if let Some(view) = views.get(active_layer) {
                let index = active_layer;
                ui.separator();
                ui.horizontal(|ui| {
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
                if ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0).text("Opacity")).changed() {
                    actions.push(("layer.set_opacity", json!({ "index": index, "opacity": opacity })));
                }
                if !channels.is_empty() {
                    egui::CollapsingHeader::new("Channel opacity").default_open(false).show(ui, |ui| {
                        for channel in &channels {
                            let mut value = view.channel_opacity.iter().find(|(stored, _)| stored == channel).map(|(_, value)| *value).unwrap_or(1.0);
                            ui.horizontal(|ui| {
                                ui.label(channel.label());
                                if ui.add(egui::Slider::new(&mut value, 0.0..=1.0)).changed() {
                                    actions.push(("layer.set_channel_opacity", json!({ "index": index, "channel": channel.id(), "opacity": value })));
                                }
                                if ui.small_button("reset").clicked() {
                                    actions.push(("layer.clear_channel_opacity", json!({ "index": index, "channel": channel.id() })));
                                }
                            });
                        }
                    });
                }
                if view.color.is_some() {
                    ui.separator();
                    ui.label(egui::RichText::new("Fill").color(Tokens::DARK.text_dim));
                    let source_label = match view.source {
                        "image" => "Image",
                        "procedural" => "Procedural",
                        "mesh_map" => "Mesh map",
                        _ => "Uniform",
                    };
                    let mut source = view.source;
                    egui::ComboBox::from_id_salt("sc.layer.source").selected_text(source_label).show_ui(ui, |ui| {
                        for (id, label) in [("uniform", "Uniform"), ("image", "Image"), ("procedural", "Procedural"), ("mesh_map", "Mesh map")] {
                            if ui.selectable_label(view.source == id, label).clicked() {
                                source = id;
                            }
                        }
                    });
                    if source != view.source {
                        let image_filter = || rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "tga", "tif", "tiff", "exr", "hdr", "bmp"]).pick_file();
                        match source {
                            "uniform" => actions.push(("layer.set_fill_source", json!({ "index": index, "source": "uniform" }))),
                            "image" => {
                                if let Some(path) = image_filter() {
                                    actions.push(("layer.set_fill_image", json!({ "index": index, "path": path.to_string_lossy() })));
                                }
                            }
                            "procedural" => actions.push(("layer.set_fill_source", json!({ "index": index, "source": "procedural", "generator": "perlin" }))),
                            _ => actions.push(("layer.set_fill_source", json!({ "index": index, "source": "mesh_map", "mesh_map": "curvature" }))),
                        }
                    }
                    if view.source == "image"
                        && ui.button("Replace image…").clicked()
                        && let Some(path) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "tga", "tif", "tiff", "exr", "hdr", "bmp"]).pick_file()
                    {
                        actions.push(("layer.set_fill_image", json!({ "index": index, "path": path.to_string_lossy() })));
                    }
                    if view.source == "procedural" {
                        let generator = view.generator.unwrap_or("perlin");
                        let selected = Generator::from_id(generator).map(|value| value.label()).unwrap_or("Perlin Noise");
                        egui::ComboBox::from_id_salt("sc.layer.generator").selected_text(selected).show_ui(ui, |ui| {
                            for id in Generator::IDS {
                                let label = Generator::from_id(id).map(|value| value.label()).unwrap_or(id);
                                if ui.selectable_label(generator == id, label).clicked() {
                                    actions.push(("layer.set_fill_source", json!({ "index": index, "source": "procedural", "generator": id })));
                                }
                            }
                        });
                    }
                    if view.source == "mesh_map" {
                        let map = view.mesh_map.unwrap_or(MeshMapKind::Curvature);
                        egui::ComboBox::from_id_salt("sc.layer.mesh_map").selected_text(map.label()).show_ui(ui, |ui| {
                            for kind in MeshMapKind::ALL {
                                if ui.selectable_label(map == kind, kind.label()).clicked() {
                                    actions.push(("layer.set_fill_source", json!({ "index": index, "source": "mesh_map", "mesh_map": kind.id() })));
                                }
                            }
                        });
                    }
                    if view.source == "uniform" {
                        if let Some(color) = view.color {
                            let mut rgba = color;
                            ui.horizontal(|ui| {
                                ui.label("Color");
                                if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                                    actions.push(("layer.set_fill_color", json!({ "index": index, "color": rgba })));
                                }
                            });
                        }
                        if let Some(value) = view.value {
                            let mut scalar = value;
                            if ui.add(egui::Slider::new(&mut scalar, 0.0..=1.0).text("Value")).changed() {
                                actions.push(("layer.set_fill_value", json!({ "index": index, "value": scalar })));
                            }
                        }
                    } else {
                        let mut kind = view.projection.kind;
                        ui.horizontal(|ui| {
                            ui.label("Projection");
                            egui::ComboBox::from_id_salt("sc.layer.projection").selected_text(kind.label()).show_ui(ui, |ui| {
                                for option in ProjectionKind::ALL {
                                    if ui.selectable_label(kind == option, option.label()).clicked() {
                                        kind = option;
                                    }
                                }
                            });
                        });
                        if kind != view.projection.kind {
                            actions.push(("layer.set_fill_projection", json!({ "index": index, "kind": kind.id() })));
                        }
                        let (mut scale_x, mut scale_y) = (view.projection.scale[0], view.projection.scale[1]);
                        let mut rotation = view.projection.rotation;
                        let (mut offset_x, mut offset_y) = (view.projection.offset[0], view.projection.offset[1]);
                        let mut changed = false;
                        changed |= ui.add(egui::Slider::new(&mut scale_x, 0.05..=16.0).text("Tile X")).changed();
                        changed |= ui.add(egui::Slider::new(&mut scale_y, 0.05..=16.0).text("Tile Y")).changed();
                        changed |= ui.add(egui::Slider::new(&mut rotation, -180.0..=180.0).text("Rotation")).changed();
                        changed |= ui.add(egui::Slider::new(&mut offset_x, -1.0..=1.0).text("Offset X")).changed();
                        changed |= ui.add(egui::Slider::new(&mut offset_y, -1.0..=1.0).text("Offset Y")).changed();
                        if changed {
                            actions.push((
                                "layer.set_fill_projection",
                                json!({ "index": index, "scale_x": scale_x, "scale_y": scale_y, "rotation": rotation, "offset_x": offset_x, "offset_y": offset_y }),
                            ));
                        }
                    }
                    let mut strength = view.strength;
                    if ui.add(egui::Slider::new(&mut strength, 0.0..=4.0).text("Strength")).changed() {
                        actions.push(("layer.set_fill_strength", json!({ "index": index, "strength": strength })));
                    }
                }
                ui.horizontal(|ui| {
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
                egui::CollapsingHeader::new("Mask effects").default_open(!view.mask_effects.is_empty()).show(ui, |ui| {
                    for (effect_index, effect) in view.mask_effects.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(effect.label());
                            if ui.small_button("x").clicked() {
                                actions.push(("layer.remove_mask_effect", json!({ "index": index, "effect_index": effect_index })));
                            }
                        });
                        let params = match effect {
                            MaskEffect::Generator(generator) => generator.params(),
                            MaskEffect::Filter(filter) => filter.params(),
                            _ => Vec::new(),
                        };
                        for param in params {
                            let mut value = param.value;
                            if ui.add(egui::Slider::new(&mut value, param.min..=param.max).text(param.label)).changed() {
                                let mut object = serde_json::Map::new();
                                object.insert("index".into(), json!(index));
                                object.insert("effect_index".into(), json!(effect_index));
                                object.insert(param.key.into(), json!(value));
                                actions.push(("layer.set_mask_effect", Value::Object(object)));
                            }
                        }
                        match effect {
                            MaskEffect::Fill { value } => {
                                let mut value = *value;
                                if ui.add(egui::Slider::new(&mut value, 0.0..=1.0).text("Value")).changed() {
                                    actions.push(("layer.set_mask_effect", json!({ "index": index, "effect_index": effect_index, "value": value })));
                                }
                            }
                            MaskEffect::MeshMap { invert, .. } | MaskEffect::Image { invert, .. } => {
                                let mut value = *invert;
                                if ui.checkbox(&mut value, "Invert").changed() {
                                    actions.push(("layer.set_mask_effect", json!({ "index": index, "effect_index": effect_index, "invert": value })));
                                }
                            }
                            _ => {}
                        }
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("+ Curvature").clicked() {
                            actions.push(("layer.add_mask_effect", json!({ "index": index, "effect": "mesh_map", "mesh_map": "curvature" })));
                        }
                        if ui.button("+ Cavity").clicked() {
                            actions.push(("layer.add_mask_effect", json!({ "index": index, "effect": "mesh_map", "mesh_map": "ambient_occlusion" })));
                        }
                        if ui.button("+ Noise").clicked() {
                            actions.push(("layer.add_mask_effect", json!({ "index": index, "effect": "generator", "generator": "perlin" })));
                        }
                        if ui.button("+ Levels").clicked() {
                            actions.push(("layer.add_mask_effect", json!({ "index": index, "effect": "filter", "filter": "levels" })));
                        }
                        if ui.button("+ Invert").clicked() {
                            actions.push(("layer.add_mask_effect", json!({ "index": index, "effect": "filter", "filter": "invert" })));
                        }
                        if ui.button("+ Image…").clicked()
                            && let Some(path) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "tga", "tif", "tiff", "exr", "hdr", "bmp"]).pick_file()
                        {
                            actions.push(("layer.add_mask_effect", json!({ "index": index, "effect": "image", "path": path.to_string_lossy() })));
                        }
                    });
                });
                ui.horizontal(|ui| {
                    if ui.small_button("Save smart material").clicked() {
                        actions.push(("layer.save_smart_material", json!({ "index": index, "name": view.name.clone() })));
                    }
                    if ui.small_button("Save smart mask").clicked() {
                        actions.push(("layer.save_smart_mask", json!({ "index": index, "name": view.name.clone() })));
                    }
                });
                if part_count > 1 {
                    egui::CollapsingHeader::new("Parts").default_open(view.parts.is_some()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.small_button("All").clicked() {
                                actions.push(("layer.set_parts", json!({ "index": index, "parts": Value::Null })));
                            }
                            if ui.small_button("None").clicked() {
                                actions.push(("layer.set_parts", json!({ "index": index, "parts": [] })));
                            }
                            if ui.small_button("Invert").clicked() {
                                let parts: Vec<u32> = (0..part_count as u32)
                                    .filter(|part| !view.parts.as_ref().is_some_and(|set| set.contains(part)))
                                    .collect();
                                actions.push(("layer.set_parts", json!({ "index": index, "parts": parts })));
                            }
                        });
                        for part in 0..part_count {
                            let part = part as u32;
                            let mut on = view.parts.as_ref().is_none_or(|set| set.contains(&part));
                            if ui.checkbox(&mut on, format!("Part {part}")).changed() {
                                let mut set: Vec<u32> = view.parts.clone().unwrap_or_else(|| (0..part_count as u32).collect());
                                if on {
                                    if !set.contains(&part) {
                                        set.push(part);
                                    }
                                } else {
                                    set.retain(|existing| *existing != part);
                                }
                                actions.push(("layer.set_parts", json!({ "index": index, "parts": set })));
                            }
                        }
                    });
                }
                if let Some(effect) = view.effect {
                    ui.separator();
                    ui.label(egui::RichText::new(format!("{} parameters", effect.id())).color(Tokens::DARK.text_dim));
                    for param in effect.params() {
                        let mut value = param.value;
                        let changed = if param.integer {
                            let mut whole = param.value.round() as i32;
                            let changed = ui
                                .horizontal(|ui| {
                                    ui.label(param.label);
                                    ui.add(egui::DragValue::new(&mut whole).range(param.min as i32..=param.max as i32).speed(1.0)).changed()
                                })
                                .inner;
                            value = whole as f32;
                            changed
                        } else {
                            ui.add(egui::Slider::new(&mut value, param.min..=param.max).text(param.label)).changed()
                        };
                        if changed {
                            push_effect_param(&mut actions, index, param.key, value);
                        }
                    }
                }
            }
        });

        self.active_layer = active_layer;
        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// Properties — Paint (material + mask previews, brush).
    pub(crate) fn properties_panel(&mut self, ui: &mut egui::Ui) {
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
                layers.get(index).and_then(|layer| layer.mask.as_ref().map(|mask| mask.coverage()))
            });
            (session.texture_set_revision(), set, material, mask)
        };
        if let Some(set) = &set {
            self.update_layer_preview(ui.ctx(), revision, set, &material);
        }
        let preview = self.layer_preview.as_ref().map(|(_, handle)| handle.clone());

        let (color, size, symmetry, lazy_radius) = {
            let session = lock(&self.session);
            let brush = *session.brush();
            (brush.color, brush.screen_radius_px, brush.symmetry, brush.lazy_radius_px)
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
                let mut lazy = lazy_radius;
                if ui.add(egui::Slider::new(&mut lazy, 0.0..=128.0).fixed_decimals(0).text("Lazy mouse")).changed() {
                    actions.push(("brush.set_lazy", json!({ "radius": lazy })));
                }
                ui.horizontal(|ui| {
                    if ui.button("Brush tip…").clicked()
                        && let Some(path) =
                            rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "tga", "tif", "tiff", "bmp"]).pick_file()
                    {
                        actions.push(("brush.set_alpha", json!({ "path": path.to_string_lossy() })));
                    }
                    if ui.button("Round").clicked() {
                        actions.push(("brush.clear_alpha", Value::Null));
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.brush_preset_name).desired_width(90.0).hint_text("Preset"));
                    if ui.button("Save").clicked() {
                        actions.push(("brush.save_preset", json!({ "name": self.brush_preset_name.clone() })));
                    }
                });
            });
        });

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The assets shelf: a library selector, search, icon filter row and a thumbnail grid of
    /// materials, brushes, generators and filters.
    pub(crate) fn assets_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
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

        let (mat_name, mat_color, brush_color, preset_names, smart_material_names, smart_mask_names) = {
            let session = lock(&self.session);
            let project = session.project();
            let material = project
                .active_set()
                .and_then(|set| set.materials.first())
                .and_then(|id| project.scene.materials.get(*id as usize))
                .cloned()
                .unwrap_or_default();
            (
                material.name,
                material.base_color,
                session.brush().color,
                session.material_preset_names(),
                session.smart_material_names(),
                session.smart_mask_names(),
            )
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
        if show("material") {
            for name in &preset_names {
                if matches(name) {
                    tiles.push(AssetTile {
                        label: name.clone(),
                        tooltip: format!("Apply material preset {name}"),
                        action: Some(("shelf.apply_material", json!({ "name": name }))),
                        thumb: Thumb::Swatch([0.6, 0.6, 0.62, 1.0]),
                    });
                }
            }
        }
        if show("smart_material") {
            for name in &smart_material_names {
                if matches(name) {
                    tiles.push(AssetTile {
                        label: name.clone(),
                        tooltip: format!("Apply smart material {name}"),
                        action: Some(("shelf.apply_smart_material", json!({ "name": name }))),
                        thumb: Thumb::Gradient,
                    });
                }
            }
        }
        if show("smart_mask") {
            for name in &smart_mask_names {
                if matches(name) {
                    tiles.push(AssetTile {
                        label: name.clone(),
                        tooltip: format!("Apply smart mask {name} to the selected layer"),
                        action: Some(("shelf.apply_smart_mask", json!({ "layer": self.active_layer, "name": name }))),
                        thumb: Thumb::Gradient,
                    });
                }
            }
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
        self.ensure_generator_thumbs(ui.ctx());

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
        });
        self.generator_thumbs = thumbs;

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The bake panel: mesh-map bake settings, high-poly import and the last bake.
    pub(crate) fn bake_panel(&mut self, ui: &mut egui::Ui) {
        let (resolution, samples, high_poly, bake_revision, first_bake, log) = {
            let session = lock(&self.session);
            let first_bake = session.baked_maps().first().map(|(_, image)| (image.width, image.height, image.to_rgba8_linear()));
            (
                session.bake_settings().resolution,
                session.bake_settings().samples,
                session.high_poly().is_some(),
                session.bake_revision(),
                first_bake,
                session.bake_log().to_vec(),
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
        egui::ScrollArea::vertical().id_salt("sc.bake").show(ui, |ui| {
            ui.add_space(4.0);
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
                    && let Some(path) =
                        rfd::FileDialog::new().add_filter("3D meshes", &["obj", "gltf", "glb", "stl", "ply", "fbx", "dae"]).pick_file()
                {
                    actions.push(("file.import_highpoly", json!({ "path": path.to_string_lossy() })));
                }
                if ui.add_enabled(high_poly, egui::Button::new("Clear")).clicked() {
                    actions.push(("bake.clear_highpoly", Value::Null));
                }
            });
            if ui.button("Bake Mesh Maps").clicked() {
                actions.push(("bake.run", Value::Null));
            }
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
        });

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The post-effects panel.
    pub(crate) fn post_panel(&mut self, ui: &mut egui::Ui) {
        let (vignette, bloom) = {
            let session = lock(&self.session);
            (session.settings().vignette, session.settings().bloom)
        };
        let mut actions: Vec<(&'static str, Value)> = Vec::new();
        ui.add_space(4.0);
        let mut vignette_value = vignette;
        if ui.add(egui::Slider::new(&mut vignette_value, 0.0..=1.0).text("Vignette")).changed() {
            actions.push(("viewport.set_vignette", json!({ "value": vignette_value })));
        }
        let mut bloom_value = bloom;
        if ui.add(egui::Slider::new(&mut bloom_value, 0.0..=1.0).text("Bloom")).changed() {
            actions.push(("viewport.set_bloom", json!({ "value": bloom_value })));
        }
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

    /// The 2D UV view: the active set's base-colour composite with a UV wireframe overlay.
    pub(crate) fn uv_view_panel(&mut self, ui: &mut egui::Ui) {
        let (revision, set, material) = {
            let session = lock(&self.session);
            let project = session.project();
            let set = project.active_set().cloned();
            let material = project
                .active_set()
                .and_then(|set| set.materials.first())
                .and_then(|id| project.scene.materials.get(*id as usize))
                .cloned()
                .unwrap_or_default();
            (session.texture_set_revision(), set, material)
        };
        let Some(set) = set else {
            ui.label(egui::RichText::new("No texture set").color(Tokens::DARK.text_dim));
            return;
        };
        if self.uv_view.as_ref().is_none_or(|(cached, _)| *cached != revision) {
            let image = substancecraft_compose::evaluate_base_color(&set, &material, 256, 256);
            let size = [image.width as usize, image.height as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, &image.to_rgba8());
            let handle = ui.ctx().load_texture("sc.uv_view", color, egui::TextureOptions::LINEAR);
            self.uv_view = Some((revision, handle));
        }
        let Some(handle) = self.uv_view.as_ref().map(|(_, handle)| handle.clone()) else { return };

        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let side = rect.width().min(rect.height()).max(1.0);
        let image_rect = egui::Rect::from_min_size(rect.min, egui::vec2(side, side));
        ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, Tokens::DARK.viewport_bg);
        ui.painter().image(handle.id(), image_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);

        // UV wireframe overlay (subsampled so dense meshes stay responsive).
        let edges = {
            let session = lock(&self.session);
            let mesh = &session.project().scene.mesh;
            let triangles = mesh.indices.as_chunks::<3>().0;
            let step = (triangles.len() / 2000).max(1);
            let mut edges: Vec<[f32; 2]> = Vec::new();
            for triangle in triangles.iter().step_by(step) {
                for (i, j) in [(0usize, 1usize), (1, 2), (2, 0)] {
                    if let (Some(a), Some(b)) = (mesh.uvs.get(triangle[i] as usize), mesh.uvs.get(triangle[j] as usize)) {
                        edges.push([a.x, a.y]);
                        edges.push([b.x, b.y]);
                    }
                }
            }
            edges
        };
        let stroke = egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 48));
        for pair in edges.as_chunks::<2>().0 {
            let a = egui::pos2(image_rect.left() + pair[0][0] * side, image_rect.top() + pair[0][1] * side);
            let b = egui::pos2(image_rect.left() + pair[1][0] * side, image_rect.top() + pair[1][1] * side);
            ui.painter().line_segment([a, b], stroke);
        }
    }

    /// Recompute the composited base-colour preview when the texture set revision changes.
    pub(crate) fn update_layer_preview(&mut self, ctx: &egui::Context, revision: u64, set: &TextureSet, material: &Material) {
        if self.layer_preview.as_ref().is_some_and(|(cached, _)| *cached == revision) {
            return;
        }
        let image = {
            let session = lock(&self.session);
            let surface = if substancecraft_compose::stack_needs_surface(&set.layers) { session.eval_surface(96, 96) } else { None };
            let mesh_maps = session.eval_mesh_maps();
            let context = substancecraft_compose::EvalContext { surface: surface.as_ref(), mesh_maps: &mesh_maps };
            substancecraft_compose::evaluate_base_color_ctx(set, material, 96, 96, &context)
        };
        let size = [image.width as usize, image.height as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, &image.to_rgba8());
        let handle = ctx.load_texture("sc.layer_preview", color, egui::TextureOptions::LINEAR);
        self.layer_preview = Some((revision, handle));
    }
}
