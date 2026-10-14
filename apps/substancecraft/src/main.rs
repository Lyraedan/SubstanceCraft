//! The SubstanceCraft desktop application.

use std::sync::Arc;

use substancecraft_engine::session::{SharedSession, shared_session};
use substancecraft_ui::StudioApp;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let registry = Arc::new(substancecraft_engine::registry());
    let session = shared_session("untitled");

    if let Some(addr) = control_addr(&args) {
        start_control(&addr, Arc::clone(&registry), SharedSession::clone(&session));
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SubstanceCraft")
            .with_inner_size([1600.0, 950.0])
            .with_min_inner_size([960.0, 600.0])
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native("SubstanceCraft", options, Box::new(move |cc| Ok(Box::new(StudioApp::new(cc, registry, session)))))
}

/// The window/app icon, decoded from the embedded PNG. Falls back to the egui default if it cannot
/// be decoded (never panics).
fn app_icon() -> egui::IconData {
    match substancecraft_codecs::decode_rgba8_bytes(include_bytes!("../../../assets/icon.png")) {
        Ok((width, height, rgba)) => egui::IconData { rgba, width, height },
        Err(err) => {
            eprintln!("warning: could not decode the app icon: {err}");
            egui::IconData::default()
        }
    }
}

/// Start the JSON control channel on a background thread. Failures are reported, not fatal.
fn start_control(addr: &str, registry: Arc<substancecraft_engine::CommandRegistry>, session: SharedSession) {
    match substancecraft_automation::bind(addr) {
        Ok(listener) => {
            match listener.local_addr() {
                Ok(local) => eprintln!("control channel listening on {local}"),
                Err(_) => eprintln!("control channel listening"),
            }
            std::thread::spawn(move || {
                if let Err(err) = substancecraft_automation::serve(listener, registry, session) {
                    eprintln!("control channel stopped: {err}");
                }
            });
        }
        Err(err) => eprintln!("warning: could not start the control channel on {addr}: {err}"),
    }
}

/// Parse `--control [ADDR]` / `--control=ADDR`; default port when omitted.
fn control_addr(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(addr) = arg.strip_prefix("--control=") {
            return Some(addr.to_string());
        }
        if arg == "--control" {
            return Some(match iter.next() {
                Some(addr) if !addr.starts_with('-') => addr.clone(),
                _ => "127.0.0.1:8765".to_string(),
            });
        }
    }
    None
}
