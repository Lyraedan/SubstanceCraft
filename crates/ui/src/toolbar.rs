//! The contextual tool properties bar.
//!
//! The left side is the active tool's parameters (Painter's Size / Flow / Stroke opacity /
//! Spacing strip); the right side holds quick viewport display controls. Everything dispatches
//! the same commands the control channel uses.

use serde_json::{Value, json};
use substancecraft_engine::session::lock;
use substancecraft_paint::{Symmetry, ToolKind};
use substancecraft_render::Projection;

use crate::app::StudioApp;

/// The display-mode buttons as `(display id, label)`.
const DISPLAY_MODES: &[(&str, &str)] = &[
    ("base_color", "Base"),
    ("normal", "Normal"),
    ("roughness", "Rough"),
    ("metalness", "Metal"),
    ("uv_checker", "UV"),
    ("wireframe", "Wire"),
    ("ambient_occlusion", "AO"),
    ("height", "Height"),
    ("matcap", "Matcap"),
];

/// Draw the tool properties bar.
pub(crate) fn tool_properties_bar(app: &mut StudioApp, ui: &mut egui::Ui) {
    let (tool, brush, display, projection, grid, background) = {
        let session = lock(&app.session);
        let settings = session.settings();
        (session.tool(), *session.brush(), settings.display.id(), session.camera().projection, settings.show_grid, settings.show_background)
    };

    let mut actions: Vec<(&'static str, Value)> = Vec::new();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tool.label()).strong());
        ui.separator();

        let mut size = brush.screen_radius_px;
        ui.label("Size");
        if ui.add(egui::Slider::new(&mut size, 1.0..=256.0).fixed_decimals(0)).changed() {
            actions.push(("brush.set_screen_radius", json!({ "radius": size })));
        }
        ui.separator();

        let mut hardness = brush.hardness;
        ui.label("Hardness");
        if ui.add(egui::Slider::new(&mut hardness, 0.0..=1.0)).changed() {
            actions.push(("brush.set_hardness", json!({ "hardness": hardness })));
        }
        ui.separator();

        let mut spacing = brush.spacing;
        ui.label("Spacing");
        if ui.add(egui::Slider::new(&mut spacing, 0.01..=1.0)).changed() {
            actions.push(("brush.set_spacing", json!({ "spacing": spacing })));
        }
        ui.separator();

        let mut opacity = brush.opacity;
        ui.label("Opacity");
        if ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0)).changed() {
            actions.push(("brush.set_opacity", json!({ "opacity": opacity })));
        }

        if tool == ToolKind::Smudge {
            ui.separator();
            let mut smudge = brush.smudge;
            ui.label("Strength");
            if ui.add(egui::Slider::new(&mut smudge, 0.0..=1.0)).changed() {
                actions.push(("brush.set_smudge", json!({ "value": smudge })));
            }
        }
        if tool == ToolKind::Clone {
            ui.separator();
            let mut dx = brush.clone_offset[0];
            let mut dy = brush.clone_offset[1];
            ui.label("Offset");
            let changed_x = ui.add(egui::DragValue::new(&mut dx).speed(1.0)).changed();
            let changed_y = ui.add(egui::DragValue::new(&mut dy).speed(1.0)).changed();
            if changed_x || changed_y {
                actions.push(("brush.set_clone_offset", json!({ "dx": dx, "dy": dy })));
            }
        }

        ui.separator();
        let mut rgba = brush.color;
        if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
            actions.push(("brush.set_color", json!({ "color": rgba })));
        }
        egui::ComboBox::from_id_salt("sc.toolbar.symmetry").selected_text(brush.symmetry.label()).show_ui(ui, |ui| {
            for mode in Symmetry::ALL {
                if ui.selectable_label(mode == brush.symmetry, mode.label()).clicked() {
                    actions.push(("brush.set_symmetry", json!({ "mode": mode.id() })));
                }
            }
        });

        // Quick viewport controls, right-aligned.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Frame").clicked() {
                actions.push(("viewport.frame_all", Value::Null));
            }
            if ui.selectable_label(grid, "Grid").clicked() {
                actions.push(("viewport.set_grid", json!({ "enabled": !grid })));
            }
            if ui.selectable_label(background, "Sky").clicked() {
                actions.push(("viewport.set_background", json!({ "enabled": !background })));
            }
            egui::ComboBox::from_id_salt("sc.toolbar.projection")
                .selected_text(match projection {
                    Projection::Perspective => "Persp",
                    Projection::Orthographic => "Ortho",
                })
                .show_ui(ui, |ui| {
                    for (mode, label) in [(Projection::Perspective, "Persp"), (Projection::Orthographic, "Ortho")] {
                        if ui.selectable_label(projection == mode, label).clicked() {
                            actions.push(("viewport.set_projection", json!({ "mode": mode.id() })));
                        }
                    }
                });
            egui::ComboBox::from_id_salt("sc.toolbar.display")
                .selected_text(DISPLAY_MODES.iter().find(|(id, _)| *id == display).map(|(_, label)| *label).unwrap_or("Display"))
                .show_ui(ui, |ui| {
                    for (id, label) in DISPLAY_MODES {
                        if ui.selectable_label(display == *id, *label).clicked() {
                            actions.push(("viewport.set_display_mode", json!({ "mode": id })));
                        }
                    }
                });
        });
    });

    for (id, params) in actions {
        let outcome = app.dispatch(id, &params);
        app.report(outcome);
    }
}
