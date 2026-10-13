//! The eframe application shell.
//!
//! Thin by rule: it renders state and dispatches commands by id. It does not implement
//! document behaviour. Window close is routed through `craft-quit` so macOS exits cleanly.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use craft_settings::{Location, Settings, SettingsStore};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use substancecraft_doc::Channel;
use substancecraft_engine::command::{CommandError, CommandOutcome, CommandRegistry};
use substancecraft_engine::session::{SharedSession, lock};
use substancecraft_render::Projection;

use crate::state::UiState;
use crate::theme::Tokens;
use crate::viewport::Viewport;

/// Everything the shell persists between runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// View state (theme, panel visibility).
    pub ui: UiState,
}

impl Settings for Prefs {
    const APP_ID: &'static str = "substancecraft";
    const SCHEMA_VERSION: u32 = 1;
}

/// The display-mode buttons in the toolbar, as `(display id, label)`.
const DISPLAY_MODES: &[(&str, &str)] =
    &[("base_color", "Base"), ("normal", "Normal"), ("roughness", "Rough"), ("metalness", "Metal"), ("uv_checker", "UV"), ("wireframe", "Wire")];

/// The main application.
pub struct StudioApp {
    registry: Arc<CommandRegistry>,
    session: SharedSession,
    prefs: SettingsStore<Prefs>,
    status: String,
    viewport: Viewport,
    pending_screenshot: Option<PathBuf>,
    rename_buffer: String,
    rename_index: usize,
}

impl StudioApp {
    /// Build the app for eframe, installing the theme and app menu.
    pub fn new(cc: &eframe::CreationContext<'_>, registry: Arc<CommandRegistry>, session: SharedSession) -> Self {
        Tokens::DARK.apply(&cc.egui_ctx);

        if let Err(err) = craft_quit::install_app_menu(&cc.egui_ctx, "SubstanceCraft") {
            eprintln!("warning: could not install the macOS application menu: {err}");
        }

        let (prefs, _report) = SettingsStore::<Prefs>::open(Location::Auto);

        // Show something in the viewport immediately, through the command path.
        let mut app = Self {
            registry,
            session,
            prefs,
            status: format!("SubstanceCraft {}", env!("CARGO_PKG_VERSION")),
            viewport: Viewport::default(),
            pending_screenshot: None,
            rename_buffer: String::new(),
            rename_index: usize::MAX,
        };
        app.report(app.dispatch("scene.load_default", &Value::Null));
        app
    }

    /// Dispatch a command by id against the shared session.
    fn dispatch(&self, id: &str, params: &Value) -> Result<CommandOutcome, CommandError> {
        let mut session = lock(&self.session);
        self.registry.run(id, &mut session, params)
    }

    /// Fold a command result into the status line.
    fn report(&mut self, result: Result<CommandOutcome, CommandError>) {
        self.status = match result {
            Ok(outcome) => outcome.message.unwrap_or_else(|| "done".to_string()),
            Err(err) => err.to_string(),
        };
    }

    fn ui_state(&self) -> UiState {
        self.prefs.get().ui.clone()
    }

    fn current_display_mode(&self) -> &'static str {
        lock(&self.session).settings().display.id()
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("SubstanceCraft");
            ui.separator();

            if ui.button("Load Demo").clicked() {
                self.report(self.dispatch("scene.load_default", &Value::Null));
            }
            if ui.button("Import Mesh…").clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("3D meshes", &["obj", "gltf", "glb"]).pick_file()
            {
                self.report(self.dispatch("file.import_mesh", &json!({ "path": path.to_string_lossy() })));
            }
            if ui.button("Frame").clicked() {
                self.report(self.dispatch("viewport.frame_all", &Value::Null));
            }

            ui.separator();
            let current = self.current_display_mode();
            for (id, label) in DISPLAY_MODES {
                if ui.selectable_label(current == *id, *label).clicked() {
                    self.report(self.dispatch("viewport.set_display_mode", &json!({ "mode": id })));
                }
            }

            ui.separator();
            let projection = lock(&self.session).camera().projection;
            if ui.selectable_label(projection == Projection::Perspective, "Persp").clicked() {
                self.report(self.dispatch("viewport.set_projection", &json!({ "mode": "perspective" })));
            }
            if ui.selectable_label(projection == Projection::Orthographic, "Ortho").clicked() {
                self.report(self.dispatch("viewport.set_projection", &json!({ "mode": "orthographic" })));
            }

            ui.separator();
            let (grid, background) = {
                let session = lock(&self.session);
                (session.settings().show_grid, session.settings().show_background)
            };
            if ui.selectable_label(grid, "Grid").clicked() {
                self.report(self.dispatch("viewport.set_grid", &json!({ "enabled": !grid })));
            }
            if ui.selectable_label(background, "Sky").clicked() {
                self.report(self.dispatch("viewport.set_background", &json!({ "enabled": !background })));
            }
            if ui.button("Env…").clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("HDR environments", &["hdr", "exr"]).pick_file()
            {
                self.report(self.dispatch("viewport.set_environment", &json!({ "path": path.to_string_lossy() })));
            }
            if ui.button("Demo Env").clicked() {
                self.report(self.dispatch("viewport.use_demo_environment", &Value::Null));
            }

            ui.separator();
            if ui.button("About").clicked() {
                self.report(self.dispatch("app.version", &Value::Null));
            }
            if ui.button("Shot…").clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name("screenshot.png").save_file()
            {
                self.report(self.dispatch("ui.screenshot", &json!({ "path": path.to_string_lossy() })));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut show_sets = self.ui_state().show_texture_sets;
                let mut show_layers = self.ui_state().show_layer_stack;
                let mut show_shelf = self.ui_state().show_shelf;
                let changed = ui.toggle_value(&mut show_shelf, "Shelf").changed()
                    | ui.toggle_value(&mut show_layers, "Layers").changed()
                    | ui.toggle_value(&mut show_sets, "Sets").changed();
                if changed {
                    self.prefs.update(|prefs| {
                        prefs.ui.show_texture_sets = show_sets;
                        prefs.ui.show_layer_stack = show_layers;
                        prefs.ui.show_shelf = show_shelf;
                    });
                }
            });
        });
    }

    fn texture_sets_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.strong("Texture Sets");
        ui.separator();

        // Snapshot the project state so we never hold the lock while dispatching.
        let (sets, active, resolution, channels, tiles, isolate) = {
            let session = lock(&self.session);
            let project = session.project();
            let sets: Vec<(String, usize)> =
                project.texture_sets.iter().enumerate().map(|(index, set)| (set.name.clone(), project.triangles_for_set(index))).collect();
            let active = project.active_texture_set;
            let (resolution, channels, tiles) = project
                .active_set()
                .map(|set| (set.resolution, set.channels.clone(), set.uv_tiles.clone()))
                .unwrap_or((2048, Vec::new(), Vec::new()));
            (sets, active, resolution, channels, tiles, session.settings().isolate_active_set)
        };

        if sets.is_empty() {
            ui.label(egui::RichText::new("No project loaded").color(Tokens::DARK.text_dim));
            return;
        }

        egui::ScrollArea::vertical().max_height(140.0).id_salt("sc.sets.list").show(ui, |ui| {
            for (index, (name, triangles)) in sets.iter().enumerate() {
                if ui.selectable_label(index == active, format!("{name}  ({triangles} tris)")).clicked() {
                    self.report(self.dispatch("texture_set.select", &json!({ "index": index })));
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
                    self.report(self.dispatch("texture_set.rename", &json!({ "name": name })));
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Res");
            for value in [512u32, 1024, 2048, 4096] {
                if ui.selectable_label(resolution == value, value.to_string()).clicked() {
                    self.report(self.dispatch("texture_set.set_resolution", &json!({ "resolution": value })));
                }
            }
        });

        ui.add_space(2.0);
        ui.label("Channels");
        for channel in Channel::ALL {
            let mut enabled = channels.contains(&channel);
            if ui.checkbox(&mut enabled, channel.label()).changed() {
                self.report(self.dispatch("texture_set.set_channel", &json!({ "channel": channel.id(), "enabled": enabled })));
            }
        }

        ui.add_space(2.0);
        ui.label("UV Tiles");
        ui.horizontal_wrapped(|ui| {
            for tile in &tiles {
                if ui.small_button(format!("{} ×", tile.udim())).clicked() {
                    self.report(self.dispatch("texture_set.remove_uv_tile", &json!({ "udim": tile.udim() })));
                }
            }
            if ui.small_button("+ Tile").clicked() {
                let next = tiles.iter().map(|tile| tile.udim()).max().unwrap_or(1001).saturating_add(1);
                self.report(self.dispatch("texture_set.add_uv_tile", &json!({ "udim": next })));
            }
        });

        let mut solo = isolate;
        if ui.checkbox(&mut solo, "Solo active set").changed() {
            self.report(self.dispatch("texture_set.set_solo", &json!({ "enabled": solo })));
        }
    }

    fn shelf_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.strong("Shelf");
        ui.separator();
        ui.label(egui::RichText::new("Materials and brushes arrive in M6").color(Tokens::DARK.text_dim));
    }

    fn layer_stack_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.strong("Layer Stack");
            ui.separator();
            ui.label(egui::RichText::new("M3").color(Tokens::DARK.text_dim));
        });
        ui.separator();
        ui.label(egui::RichText::new("Fill / paint / effect layers land here").color(Tokens::DARK.text_dim));
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&self.status).color(Tokens::DARK.text_dim));
        });
    }

    /// Fulfil a queued screenshot: request one from eframe, then save the returned image.
    fn handle_screenshots(&mut self, ctx: &egui::Context) {
        if let Some(path) = lock(&self.session).take_screenshot_request() {
            self.pending_screenshot = Some(PathBuf::from(path));
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        let captured = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = captured
            && let Some(path) = self.pending_screenshot.take()
        {
            self.report(save_screenshot(&path, &image));
        }
    }
}

/// Write an egui screenshot to a PNG.
fn save_screenshot(path: &Path, image: &egui::ColorImage) -> Result<CommandOutcome, CommandError> {
    let [width, height] = image.size;
    let mut bytes = Vec::with_capacity(width.saturating_mul(height).saturating_mul(4));
    for pixel in &image.pixels {
        bytes.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), pixel.a()]);
    }
    image::save_buffer(path, &bytes, width as u32, height as u32, image::ExtendedColorType::Rgba8)
        .map_err(|err| CommandError::Failed(format!("could not save screenshot: {err}")))?;
    Ok(CommandOutcome::message(format!("screenshot saved: {}", path.display())))
}

impl eframe::App for StudioApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c = Tokens::DARK.window_bg;
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, 1.0]
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let _ = self.prefs.tick(std::time::Instant::now());
        self.handle_screenshots(ctx);
        craft_quit::on_close_request(ctx);
        // Poll session state so control-channel-driven changes appear without user input.
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::Panel::top("sc.top").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("sc.status").show(ui, |ui| self.status_bar(ui));

        if self.ui_state().show_texture_sets {
            egui::Panel::left("sc.sets").default_size(200.0).min_size(150.0).resizable(true).show(ui, |ui| self.texture_sets_panel(ui));
        }
        if self.ui_state().show_shelf {
            egui::Panel::right("sc.shelf").default_size(230.0).min_size(160.0).resizable(true).show(ui, |ui| self.shelf_panel(ui));
        }
        if self.ui_state().show_layer_stack {
            egui::Panel::bottom("sc.layers").default_size(200.0).min_size(120.0).resizable(true).show(ui, |ui| self.layer_stack_panel(ui));
        }

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| self.viewport.show(ui, frame, &self.session));
    }

    fn on_exit(&mut self) {
        let _ = self.prefs.flush();
    }
}
