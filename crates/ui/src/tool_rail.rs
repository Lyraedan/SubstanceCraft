//! The left icon tool rail.
//!
//! Selecting a paint tool dispatches `tool.set`; the paint/orbit entries switch the shell's
//! viewport interaction mode. Glyphs are placeholders for the future SVG catalog.

use serde_json::json;
use substancecraft_engine::session::lock;
use substancecraft_paint::ToolKind;

use crate::app::StudioApp;
use crate::icons;
use crate::viewport::ViewportTool;

/// Draw the vertical tool rail.
pub(crate) fn tool_rail(app: &mut StudioApp, ui: &mut egui::Ui) {
    let active_tool = lock(&app.session).tool();
    let painting = app.tool == ViewportTool::Paint;

    let mut set_view: Option<ViewportTool> = None;
    let mut select_tool: Option<ToolKind> = None;

    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        if rail_button(ui, icons::glyph("paint"), painting, "Paint (left-drag paints)") {
            set_view = Some(ViewportTool::Paint);
        }
        if rail_button(ui, icons::glyph("orbit"), app.tool == ViewportTool::Orbit, "Orbit camera (left-drag orbits)") {
            set_view = Some(ViewportTool::Orbit);
        }
        ui.add_space(4.0);
        ui.separator();
        for tool in ToolKind::ALL {
            let selected = painting && tool == active_tool;
            if rail_button(ui, icons::glyph(tool.id()), selected, tool.label()) {
                select_tool = Some(tool);
            }
        }
    });

    if let Some(view) = set_view {
        app.tool = view;
    }
    if let Some(tool) = select_tool {
        app.tool = ViewportTool::Paint;
        let outcome = app.dispatch("tool.set", &json!({ "tool": tool.id() }));
        app.report(outcome);
    }
}

/// A square rail button with a tooltip.
fn rail_button(ui: &mut egui::Ui, glyph: &str, selected: bool, tooltip: &str) -> bool {
    ui.add_sized([30.0, 30.0], egui::Button::new(egui::RichText::new(glyph).size(16.0)).selected(selected)).on_hover_text(tooltip).clicked()
}
