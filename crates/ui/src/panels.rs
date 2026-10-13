//! The dockable panel bodies: right dock (texture set list, layers, settings, properties), the
//! bottom assets shelf and the status bar.
//!
//! Mirrors Substance 3D Painter's right column and bottom shelf. Views read tokens and dispatch
//! commands; they never mutate the document directly.

use serde_json::{Value, json};
use substancecraft_bake::BakeMap;
use substancecraft_doc::{BlendMode, Channel, Filter, Generator, LayerKind, Material, Shader, TextureSet};
use substancecraft_engine::session::lock;

use crate::app::StudioApp;
use crate::icons;
use crate::state::RightTab;
use crate::theme::Tokens;

/// The assets shelf's filter tabs as `(label, section id)`.
const ASSET_FILTERS: &[(&str, &str)] = &[("All", "all"), ("Generators", "generators"), ("Filters", "filters"), ("Bake", "bake"), ("Post", "post")];

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

impl StudioApp {
    /// The right dock's top panel: the texture set list.
    pub(crate) fn texture_set_list_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.strong("Texture Set List");
        });
        ui.separator();

        let (sets, active, resolution, tiles, isolate) = {
            let session = lock(&self.session);
            let project = session.project();
            let sets: Vec<(String, usize)> =
                project.texture_sets.iter().enumerate().map(|(index, set)| (set.name.clone(), project.triangles_for_set(index))).collect();
            let active = project.active_texture_set;
            let (resolution, tiles) = project.active_set().map(|set| (set.resolution, set.uv_tiles.clone())).unwrap_or((2048, Vec::new()));
            (sets, active, resolution, tiles, session.settings().isolate_active_set)
        };

        if sets.is_empty() {
            ui.label(egui::RichText::new("No project loaded").color(Tokens::DARK.text_dim));
            return;
        }

        let mut actions: Vec<(&'static str, Value)> = Vec::new();
        egui::ScrollArea::vertical().max_height(140.0).id_salt("sc.sets.list").show(ui, |ui| {
            for (index, (name, triangles)) in sets.iter().enumerate() {
                if ui.selectable_label(index == active, format!("{name}  ({triangles} tris)")).clicked() {
                    actions.push(("texture_set.select", json!({ "index": index })));
                }
            }
        });

        ui.separator();
        if self.rename_index != active {
            self.rename_index = active;
            self.rename_buffer = sets.get(active).map(|(name, _)| name.clone()).unwrap_or_default();
        }
        ui.horizontal(|ui| {
            ui.label("Name");
            let response = ui.add(egui::TextEdit::singleline(&mut self.rename_buffer).desired_width(f32::INFINITY));
            if response.lost_focus() {
                let name = self.rename_buffer.trim().to_string();
                if !name.is_empty() {
                    actions.push(("texture_set.rename", json!({ "name": name })));
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

        ui.add_space(2.0);
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

        let mut solo = isolate;
        if ui.checkbox(&mut solo, "Solo active set").changed() {
            actions.push(("texture_set.set_solo", json!({ "enabled": solo })));
        }

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The right dock's `Texture Set Settings` tab: channels, resolution and shader.
    pub(crate) fn texture_set_settings_panel(&mut self, ui: &mut egui::Ui) {
        let (channels, shader, params) = {
            let session = lock(&self.session);
            session.project().active_set().map(|set| (set.channels.clone(), set.shader, set.shader_params)).unwrap_or_default()
        };

        let mut actions: Vec<(&'static str, Value)> = Vec::new();

        ui.add_space(2.0);
        ui.label("Channels");
        for channel in Channel::ALL {
            let mut enabled = channels.contains(&channel);
            if ui.checkbox(&mut enabled, channel.label()).changed() {
                actions.push(("texture_set.set_channel", json!({ "channel": channel.id(), "enabled": enabled })));
            }
        }

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
        let current = self.ui_state().right_tab;
        let mut chosen = current;
        ui.horizontal(|ui| {
            for tab in [RightTab::Layers, RightTab::Settings] {
                if ui.selectable_label(current == tab, tab.label()).clicked() {
                    chosen = tab;
                }
            }
        });
        if chosen != current {
            self.update_ui(|state| state.right_tab = chosen);
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
            if ui.button(format!("{} Fill", icons::glyph("add"))).clicked() {
                self.report(self.dispatch("layer.add_fill", &Value::Null));
                self.active_layer = usize::MAX;
            }
            if ui.button(format!("{} Group", icons::glyph("add"))).clicked() {
                self.report(self.dispatch("layer.add_group", &Value::Null));
                self.active_layer = usize::MAX;
            }
            let has_selection = self.active_layer != usize::MAX;
            if ui.add_enabled(has_selection, egui::Button::new(icons::glyph("delete"))).on_hover_text("Delete layer").clicked() {
                self.report(self.dispatch("layer.delete", &json!({ "index": self.active_layer })));
            }
            if ui.add_enabled(has_selection, egui::Button::new(icons::glyph("up"))).on_hover_text("Move up").clicked() {
                self.report(self.dispatch("layer.move", &json!({ "index": self.active_layer, "direction": "up" })));
                self.active_layer = self.active_layer.saturating_add(1);
            }
            if ui.add_enabled(has_selection, egui::Button::new(icons::glyph("down"))).on_hover_text("Move down").clicked() {
                self.report(self.dispatch("layer.move", &json!({ "index": self.active_layer, "direction": "down" })));
                self.active_layer = self.active_layer.saturating_sub(1);
            }
            if ui.add_enabled(has_selection, egui::Button::new(icons::glyph("group"))).on_hover_text("Group layer").clicked() {
                self.report(self.dispatch("layer.group", &json!({ "index": self.active_layer })));
            }
            if ui.add_enabled(can_undo, egui::Button::new(icons::glyph("undo"))).on_hover_text("Undo").clicked() {
                self.report(self.dispatch("edit.undo", &Value::Null));
            }
            if ui.add_enabled(can_redo, egui::Button::new(icons::glyph("redo"))).on_hover_text("Redo").clicked() {
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
                        if ui.small_button(icons::glyph(if view.visible { "visibility" } else { "hidden" })).clicked() {
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

    /// The right dock's bottom section: Properties (a composited preview and the active brush).
    pub(crate) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.strong("Properties");
        ui.separator();

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
            if let Some(handle) = &preview {
                ui.add(egui::Image::new(handle).fit_to_exact_size(egui::vec2(72.0, 72.0)));
            }
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

    /// The bottom assets shelf: a searchable, filterable list of generators, filters, bakers and
    /// post-effects.
    pub(crate) fn assets_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Assets");
            ui.separator();
            ui.label(icons::glyph("search"));
            ui.add(egui::TextEdit::singleline(&mut self.asset_search).desired_width(160.0).hint_text("Search"));
            ui.separator();
            for (index, (label, _)) in ASSET_FILTERS.iter().enumerate() {
                if ui.selectable_label(self.asset_filter == index, *label).clicked() {
                    self.asset_filter = index;
                }
            }
        });
        ui.separator();

        let filter = ASSET_FILTERS.get(self.asset_filter).map(|(_, id)| *id).unwrap_or("all");
        let query = self.asset_search.trim().to_ascii_lowercase();
        let matches = |label: &str| query.is_empty() || label.to_ascii_lowercase().contains(&query);

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
        egui::ScrollArea::vertical().id_salt("sc.assets").show(ui, |ui| {
            if (filter == "all" || filter == "generators") && !Generator::IDS.is_empty() {
                ui.add_space(4.0);
                ui.strong("Generators");
                ui.horizontal_wrapped(|ui| {
                    for id in Generator::IDS {
                        let label = Generator::from_id(id).map(|generator| generator.label()).unwrap_or(id);
                        if matches(label) && ui.add_sized([92.0, 26.0], egui::Button::new(label)).clicked() {
                            actions.push(("layer.add_generator", json!({ "generator": id })));
                        }
                    }
                });
            }

            if (filter == "all" || filter == "filters") && !Filter::IDS.is_empty() {
                ui.add_space(4.0);
                ui.strong("Filters");
                ui.horizontal_wrapped(|ui| {
                    for id in Filter::IDS {
                        let label = Filter::from_id(id).map(|filter| filter.label()).unwrap_or(id);
                        if matches(label) && ui.add_sized([92.0, 26.0], egui::Button::new(label)).clicked() {
                            actions.push(("layer.add_filter", json!({ "filter": id })));
                        }
                    }
                });
            }

            if filter == "all" || filter == "bake" {
                ui.add_space(4.0);
                ui.separator();
                ui.strong("Bake");
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
            }

            if filter == "all" || filter == "post" {
                ui.add_space(4.0);
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

        for (id, params) in actions {
            let outcome = self.dispatch(id, &params);
            self.report(outcome);
        }
    }

    /// The bottom status bar: the latest status message and the product version.
    pub(crate) fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&self.status).color(Tokens::DARK.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).color(Tokens::DARK.text_dim));
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
