//! The in-window menu bar.
//!
//! It is built from the command registry (grouped by each command's `menu_path`) plus a
//! shell-owned `Window` menu that toggles pure view state. Menus that would require a parameter
//! or a file dialog are handled by the shell's command runner; low-level and introspection
//! commands are hidden here because they are driven by panels or by automation.

use crate::app::StudioApp;

/// Top-level menus, in order (mirroring Substance 3D Painter minus its scripting menus).
const MENU_ORDER: &[&str] = &["File", "Edit", "View", "Bake", "Window", "Help"];

/// Command-id prefixes that belong to panels/automation rather than the human menu.
const HIDDEN_PREFIXES: &[&str] = &["layer.", "texture_set.", "shader.", "brush.", "paint.", "tool."];

/// Individual commands kept out of the menu (param-driven, introspection or developer actions).
const HIDDEN_IDS: &[&str] = &[
    "viewport.state",
    "viewport.status",
    "viewport.set_exposure",
    "viewport.set_projection",
    "viewport.set_display_mode",
    "viewport.set_vignette",
    "viewport.set_bloom",
    "session.info",
    "scene.info",
    "scene.clear",
    "app.capabilities",
    "file.autosave",
    "file.recover",
    "file.info",
    "bake.list",
    "bake.set_resolution",
    "bake.set_samples",
    "bake.save",
    "layer.list",
    "texture_set.list",
];

/// One registry command reduced to what the menu needs.
struct MenuCommand {
    id: &'static str,
    label: &'static str,
    shortcut: Option<&'static str>,
    top: &'static str,
}

/// Whether a command is shown in the menu bar.
fn is_visible(id: &str, menu_path: &str) -> bool {
    !menu_path.is_empty() && !HIDDEN_IDS.contains(&id) && !HIDDEN_PREFIXES.iter().any(|prefix| id.starts_with(prefix))
}

/// The first segment of a menu path (`"View > Environment"` → `"View"`).
fn top_level(menu_path: &str) -> &str {
    menu_path.split(" > ").next().unwrap_or(menu_path)
}

/// Draw the menu bar.
pub(crate) fn menu_bar(app: &mut StudioApp, ui: &mut egui::Ui) {
    let commands: Vec<MenuCommand> = app
        .registry
        .ids()
        .filter_map(|id| app.registry.get(id).map(|spec| (id, spec)))
        .filter(|(id, spec)| is_visible(id, spec.menu_path))
        .map(|(id, spec)| MenuCommand { id, label: spec.label, shortcut: spec.shortcut, top: top_level(spec.menu_path) })
        .collect();

    egui::MenuBar::new().ui(ui, |ui| {
        for &level in MENU_ORDER {
            ui.menu_button(level, |ui| {
                if level == "Window" {
                    window_toggles(app, ui);
                    if commands.iter().any(|command| command.top == level) {
                        ui.separator();
                    }
                }
                let mut clicked: Option<&'static str> = None;
                for command in commands.iter().filter(|command| command.top == level) {
                    let button = match command.shortcut {
                        Some(shortcut) => egui::Button::new(command.label).shortcut_text(shortcut),
                        None => egui::Button::new(command.label),
                    };
                    if ui.add(button).clicked() {
                        clicked = Some(command.id);
                        ui.close();
                    }
                }
                if let Some(id) = clicked {
                    app.run_menu_command(ui.ctx(), id);
                }
            });
        }
    });
}

/// The shell-owned `Window` menu: pure view state, never document state.
fn window_toggles(app: &mut StudioApp, ui: &mut egui::Ui) {
    let mut state = app.ui_state();
    let mut changed = ui.checkbox(&mut state.show_texture_sets, "Texture Set List").changed();
    changed |= ui.checkbox(&mut state.show_layer_stack, "Layers & Settings").changed();
    changed |= ui.checkbox(&mut state.show_shelf, "Assets Shelf").changed();
    changed |= ui.checkbox(&mut state.show_tool_properties, "Tool Properties").changed();
    changed |= ui.checkbox(&mut state.show_tool_rail, "Tool Rail").changed();
    if changed {
        app.update_ui(|ui_state| *ui_state = state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_takes_the_first_segment() {
        assert_eq!(top_level("View > Environment"), "View");
        assert_eq!(top_level("File"), "File");
    }

    #[test]
    fn low_level_and_introspection_commands_are_hidden() {
        assert!(!is_visible("layer.add_fill", "Window"));
        assert!(!is_visible("brush.set_color", "Brush"));
        assert!(!is_visible("session.info", "Window"));
        assert!(!is_visible("viewport.state", "View"));
        assert!(!is_visible("app.version", ""));
    }

    #[test]
    fn user_facing_commands_are_visible() {
        assert!(is_visible("file.save", "File"));
        assert!(is_visible("edit.undo", "Edit"));
        assert!(is_visible("viewport.frame_all", "View"));
        assert!(is_visible("bake.run", "Bake"));
    }
}
