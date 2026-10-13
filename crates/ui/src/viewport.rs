//! The 3D viewport panel: renders the scene offscreen with the wgpu renderer and shows the result
//! as an egui image, and maps pointer input onto the session's orbit camera.

use substancecraft_engine::session::{SharedSession, lock};
use substancecraft_render::ViewportRenderer;

/// The viewport panel state: the renderer, its registered egui texture and the camera input.
#[derive(Default)]
pub struct Viewport {
    renderer: Option<ViewportRenderer>,
    texture_id: Option<egui::TextureId>,
    uploaded_size: (u32, u32),
}

impl Viewport {
    /// Draw the viewport into `ui`, rendering the session's scene and handling camera input.
    pub fn show(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, session: &SharedSession) {
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

        self.handle_camera(ui, &response, rect, session);

        let Some(render_state) = frame.wgpu_render_state() else {
            ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, crate::theme::Tokens::DARK.viewport_bg);
            return;
        };

        let pixels_per_point = ui.ctx().pixels_per_point();
        let width = (rect.width() * pixels_per_point).round().max(1.0) as u32;
        let height = (rect.height() * pixels_per_point).round().max(1.0) as u32;

        if self.renderer.is_none() {
            self.renderer = Some(ViewportRenderer::new(&render_state.device));
        }

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

        let Some(texture_id) = self.texture_id else { return };
        ui.painter().image(texture_id, rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        ui.painter().rect_stroke(rect, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, crate::theme::Tokens::DARK.border), egui::StrokeKind::Inside);
        ui.painter().text(
            rect.left_bottom() + egui::vec2(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{}×{}", self.uploaded_size.0, self.uploaded_size.1),
            egui::FontId::monospace(11.0),
            crate::theme::Tokens::DARK.text_dim,
        );
    }

    fn handle_camera(&mut self, ui: &egui::Ui, response: &egui::Response, rect: egui::Rect, session: &SharedSession) {
        let pointer_delta = ui.input(|input| input.pointer.delta());
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);

        let mut session = lock(session);
        if response.dragged_by(egui::PointerButton::Primary) {
            session.camera_mut().orbit(pointer_delta.x, pointer_delta.y);
        } else if response.dragged_by(egui::PointerButton::Secondary) || response.dragged_by(egui::PointerButton::Middle) {
            session.camera_mut().pan(pointer_delta.x, pointer_delta.y, rect.height());
        }
        if response.hovered() && scroll.abs() > 0.0 {
            session.camera_mut().dolly(scroll * 0.0015);
        }
    }
}
