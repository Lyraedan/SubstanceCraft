//! The eframe application shell.
//!
//! Thin by rule: it renders state and dispatches commands by id. It does not implement
//! document behaviour. The workspace mirrors Substance 3D Painter's frame: a menu bar, a
//! contextual tool properties bar, a left tool rail, a right dock, a bottom assets shelf and a
//! status bar around the 3D viewport. Window close is routed through `craft-quit` so macOS exits
//! cleanly.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use craft_settings::{Location, Settings, SettingsStore};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use substancecraft_engine::command::{CommandError, CommandOutcome, CommandRegistry};
use substancecraft_engine::session::{SharedSession, lock};
use substancecraft_engine::workspace::Workspace;

use crate::state::{ThemeChoice, UiState};
use crate::theme::Tokens;
use crate::viewport::{Viewport, ViewportAction, ViewportTool};
use crate::{menu, tool_rail, toolbar};

/// Everything the shell persists between runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// View state (theme, panel visibility, dock selection).
    pub ui: UiState,
}

impl Settings for Prefs {
    const APP_ID: &'static str = "substancecraft";
    const SCHEMA_VERSION: u32 = 1;
}

/// The main application.
pub struct StudioApp {
    pub(crate) registry: Arc<CommandRegistry>,
    pub(crate) session: SharedSession,
    pub(crate) prefs: SettingsStore<Prefs>,
    pub(crate) status: String,
    pub(crate) viewport: Viewport,
    pending_screenshot: Option<PathBuf>,
    pub(crate) rename_buffer: String,
    pub(crate) rename_index: usize,
    pub(crate) active_layer: usize,
    pub(crate) layer_preview: Option<(u64, egui::TextureHandle)>,
    pub(crate) bake_preview: Option<(u64, egui::TextureHandle)>,
    pub(crate) tool: ViewportTool,
    pub(crate) asset_search: String,
    pub(crate) asset_filter: usize,
    pub(crate) asset_library: usize,
    pub(crate) generator_thumbs: std::collections::HashMap<&'static str, egui::TextureHandle>,
    window_title: String,
    last_autosave: Instant,
    last_workspace: Workspace,
}

impl StudioApp {
    /// Build the app for eframe, installing the theme and app menu.
    pub fn new(cc: &eframe::CreationContext<'_>, registry: Arc<CommandRegistry>, session: SharedSession) -> Self {
        crate::icons::install_loaders(&cc.egui_ctx);

        if let Err(err) = craft_quit::install_app_menu(&cc.egui_ctx, "SubstanceCraft") {
            eprintln!("warning: could not install the macOS application menu: {err}");
        }

        let (prefs, _report) = SettingsStore::<Prefs>::open(Location::Auto);
        prefs.get().ui.theme.tokens().apply(&cc.egui_ctx);

        let mut app = Self {
            registry,
            session,
            prefs,
            status: format!("SubstanceCraft {}", env!("CARGO_PKG_VERSION")),
            viewport: Viewport::default(),
            pending_screenshot: None,
            rename_buffer: String::new(),
            rename_index: usize::MAX,
            active_layer: usize::MAX,
            layer_preview: None,
            bake_preview: None,
            tool: ViewportTool::default(),
            asset_search: String::new(),
            asset_filter: 0,
            asset_library: 0,
            generator_thumbs: std::collections::HashMap::new(),
            window_title: String::new(),
            last_autosave: Instant::now(),
            last_workspace: Workspace::default(),
        };
        // The session's workspace is the runtime source of truth; seed it from the saved prefs.
        let workspace = app.ui_state().to_workspace();
        lock(&app.session).set_workspace(workspace);
        app.last_workspace = workspace;
        app.report(app.dispatch("scene.load_default", &Value::Null));
        app
    }

    /// Dispatch a command by id against the shared session.
    pub(crate) fn dispatch(&self, id: &str, params: &Value) -> Result<CommandOutcome, CommandError> {
        let mut session = lock(&self.session);
        self.registry.run(id, &mut session, params)
    }

    /// Fold a command result into the status line.
    pub(crate) fn report(&mut self, result: Result<CommandOutcome, CommandError>) {
        self.status = match result {
            Ok(outcome) => outcome.message.unwrap_or_else(|| "done".to_string()),
            Err(err) => err.to_string(),
        };
    }

    /// A snapshot of the persisted view state.
    pub(crate) fn ui_state(&self) -> UiState {
        self.prefs.get().ui.clone()
    }

    /// Mutate the persisted view state.
    pub(crate) fn update_ui(&mut self, edit: impl FnOnce(&mut UiState)) {
        self.prefs.update(|prefs| edit(&mut prefs.ui));
    }

    /// Dispatch a command and fold the result into the status line.
    pub(crate) fn dispatch_and_report(&mut self, id: &str, params: &Value) {
        let outcome = self.dispatch(id, params);
        self.report(outcome);
    }

    /// Switch the theme and apply it immediately.
    pub(crate) fn set_theme(&mut self, ctx: &egui::Context, choice: ThemeChoice) {
        self.update_ui(|state| state.theme = choice);
        choice.tokens().apply(ctx);
    }

    /// The currently selected theme.
    pub(crate) fn theme(&self) -> ThemeChoice {
        self.ui_state().theme
    }

    /// Dispatch a file command with a chosen path.
    fn dispatch_path(&mut self, id: &str, path: PathBuf) {
        let outcome = self.dispatch(id, &json!({ "path": path.to_string_lossy() }));
        self.report(outcome);
    }

    /// Run a command chosen from a menu, handling the ones that need a file dialog or context.
    pub(crate) fn run_menu_command(&mut self, ctx: &egui::Context, id: &str) {
        match id {
            "app.quit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            "file.open" => {
                if let Some(path) = rfd::FileDialog::new().add_filter("SubstanceCraft project", &["sccraft"]).pick_file() {
                    self.dispatch_path("file.open", path);
                }
            }
            "file.save_as" => {
                if let Some(path) =
                    rfd::FileDialog::new().add_filter("SubstanceCraft project", &["sccraft"]).set_file_name("untitled.sccraft").save_file()
                {
                    self.dispatch_path("file.save_as", path);
                }
            }
            "file.import_mesh" => {
                if let Some(path) = rfd::FileDialog::new().add_filter("3D meshes", &["obj", "gltf", "glb"]).pick_file() {
                    self.dispatch_path("file.import_mesh", path);
                }
            }
            "file.export_textures" => {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let outcome = self.dispatch("file.export_textures", &json!({ "dir": dir.to_string_lossy(), "resolution": 1024 }));
                    self.report(outcome);
                }
            }
            "ui.screenshot" => {
                if let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).set_file_name("screenshot.png").save_file() {
                    self.dispatch_path("ui.screenshot", path);
                }
            }
            "viewport.set_environment" => {
                if let Some(path) = rfd::FileDialog::new().add_filter("HDR environments", &["hdr", "exr"]).pick_file() {
                    self.dispatch_path("viewport.set_environment", path);
                }
            }
            "viewport.set_grid" => {
                let enabled = !lock(&self.session).settings().show_grid;
                let outcome = self.dispatch("viewport.set_grid", &json!({ "enabled": enabled }));
                self.report(outcome);
            }
            "viewport.set_background" => {
                let enabled = !lock(&self.session).settings().show_background;
                let outcome = self.dispatch("viewport.set_background", &json!({ "enabled": enabled }));
                self.report(outcome);
            }
            _ => {
                let outcome = self.dispatch(id, &Value::Null);
                self.report(outcome);
            }
        }
    }

    /// Turn a viewport paint action into a command.
    fn handle_paint_action(&mut self, action: ViewportAction) {
        match action {
            ViewportAction::StrokeBegin { u, v, scale } => {
                self.report(self.dispatch("paint.stroke_begin", &json!({ "u": u, "v": v, "scale": scale })))
            }
            ViewportAction::StrokeExtend { u, v } => {
                let _ = self.dispatch("paint.stroke_extend", &json!({ "u": u, "v": v }));
            }
            ViewportAction::StrokeEnd => {
                let _ = self.dispatch("paint.stroke_end", &Value::Null);
            }
            ViewportAction::Polyfill { triangle } => self.report(self.dispatch("paint.polyfill", &json!({ "triangle": triangle }))),
            ViewportAction::SelectTextureSet { index } => {
                self.report(self.dispatch("texture_set.select", &json!({ "index": index })));
            }
        }
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

        // Keep the window title in sync with the project path and dirty state.
        let title = {
            let session = lock(&self.session);
            format!("SubstanceCraft — {}", session.project_title())
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }

        // Autosave periodically while there are unsaved changes.
        if self.last_autosave.elapsed() >= Duration::from_secs(60) {
            self.last_autosave = Instant::now();
            if lock(&self.session).is_dirty() {
                let _ = self.dispatch("file.autosave", &Value::Null);
            }
        }

        // Persist workspace changes (panel toggles, dock tab) made via commands or the menu.
        let workspace = lock(&self.session).workspace();
        if workspace != self.last_workspace {
            self.last_workspace = workspace;
            self.update_ui(|ui| ui.apply_workspace(&workspace));
        }

        craft_quit::on_close_request(ctx);
        // Poll session state so control-channel-driven changes appear without user input.
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let workspace = lock(&self.session).workspace();

        // Top chrome: the menu bar, then the contextual tool properties bar.
        egui::Panel::top("sc.menu")
            .frame(egui::Frame::NONE.fill(Tokens::DARK.menu_bg).inner_margin(egui::Margin::symmetric(6, 2)))
            .show(ui, |ui| menu::menu_bar(self, ui));
        if workspace.tool_properties {
            egui::Panel::top("sc.tools").show(ui, |ui| toolbar::tool_properties_bar(self, ui));
        }

        // Bottom chrome: the status bar, then the assets shelf (spans the full width).
        egui::Panel::bottom("sc.status").show(ui, |ui| self.status_bar(ui));
        if workspace.assets {
            egui::Panel::bottom("sc.assets").default_size(220.0).min_size(120.0).resizable(true).show(ui, |ui| self.assets_panel(ui));
        }

        // Side chrome, above the shelf.
        if workspace.tool_rail {
            egui::Panel::left("sc.rail")
                .exact_size(40.0)
                .resizable(false)
                .frame(egui::Frame::NONE.fill(Tokens::DARK.rail_bg).inner_margin(egui::Margin::symmetric(2, 4)))
                .show(ui, |ui| tool_rail::tool_rail(self, ui));
        }
        if workspace.texture_sets || workspace.layers {
            egui::Panel::right("sc.right").default_size(300.0).min_size(220.0).resizable(true).show(ui, |ui| {
                if workspace.texture_sets {
                    self.texture_set_list_panel(ui);
                }
                if workspace.layers {
                    if workspace.texture_sets {
                        ui.separator();
                    }
                    self.right_section(ui);
                    ui.separator();
                    self.properties_panel(ui);
                }
            });
        }

        let mut action = None;
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            action = self.viewport.show(ui, frame, &self.session, self.tool);
        });
        if let Some(action) = action {
            self.handle_paint_action(action);
        }
    }

    fn on_exit(&mut self) {
        let _ = self.prefs.flush();
    }
}
