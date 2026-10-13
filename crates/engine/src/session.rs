//! The editing session: the mutable state commands act on.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use substancecraft_codecs::ImageF32;
use substancecraft_doc::Scene;
use substancecraft_render::{OrbitCamera, RenderSettings};

/// A session shared between the UI thread and background servers (control channel, MCP).
pub type SharedSession = Arc<Mutex<Session>>;

/// Wrap a fresh session for sharing across threads.
pub fn shared_session(label: impl Into<String>) -> SharedSession {
    Arc::new(Mutex::new(Session::new(label)))
}

/// Lock a shared session, recovering from poisoning rather than panicking: a panic in one
/// command must never take down the editor.
pub fn lock(session: &SharedSession) -> MutexGuard<'_, Session> {
    session.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A single editing session. Holds the open project (for now, the scene), the viewport camera
/// and the viewport render settings.
#[derive(Debug)]
pub struct Session {
    /// A display label for the session (project name, or `untitled`).
    label: String,
    /// Bumped on every mutation, so caches can be invalidated by revision.
    revision: u64,
    /// A monotonically increasing status message shown in the status bar.
    status: Option<String>,
    /// The imported scene the viewport draws.
    scene: Scene,
    /// Bumped only when the scene changes, so the renderer re-uploads geometry.
    scene_revision: u64,
    /// The linear HDR environment for image-based lighting, if any.
    environment: Option<Arc<ImageF32>>,
    /// Bumped when the environment changes, so the renderer re-uploads it.
    environment_revision: u64,
    /// The viewport camera.
    camera: OrbitCamera,
    /// The viewport render settings (display mode, exposure, tone mapping).
    settings: RenderSettings,
    /// A pending request to save a window screenshot to this path (handled by the shell).
    screenshot_request: Option<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new("untitled")
    }
}

impl Session {
    /// A new, empty session.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            revision: 0,
            status: None,
            scene: Scene::default(),
            scene_revision: 1,
            environment: None,
            environment_revision: 1,
            camera: OrbitCamera::default(),
            settings: RenderSettings::default(),
            screenshot_request: None,
        }
    }

    /// The session label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Rename the session.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    /// The current revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Mark the session as changed, bumping the revision.
    pub fn touch(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    /// The latest status message, if any.
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// Set the status message.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = Some(status.into());
    }

    /// The current scene.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Replace the scene and bump the revision.
    pub fn set_scene(&mut self, scene: Scene) {
        self.scene = scene;
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.touch();
    }

    /// Bumped when the scene changes, so the renderer re-uploads geometry.
    pub fn scene_revision(&self) -> u64 {
        self.scene_revision
    }

    /// The current environment map, if any.
    pub fn environment(&self) -> Option<&Arc<ImageF32>> {
        self.environment.as_ref()
    }

    /// Bumped when the environment changes, so the renderer re-uploads it.
    pub fn environment_revision(&self) -> u64 {
        self.environment_revision
    }

    /// Replace (or clear) the environment map.
    pub fn set_environment(&mut self, environment: Option<ImageF32>) {
        self.environment = environment.map(Arc::new);
        self.environment_revision = self.environment_revision.saturating_add(1);
        self.touch();
    }

    /// The viewport camera.
    pub fn camera(&self) -> &OrbitCamera {
        &self.camera
    }

    /// The viewport camera, mutably. Callers should `touch` after changing it.
    pub fn camera_mut(&mut self) -> &mut OrbitCamera {
        &mut self.camera
    }

    /// The viewport render settings.
    pub fn settings(&self) -> &RenderSettings {
        &self.settings
    }

    /// The viewport render settings, mutably. Callers should `touch` after changing them.
    pub fn settings_mut(&mut self) -> &mut RenderSettings {
        &mut self.settings
    }

    /// Queue a screenshot request; the shell captures the next frame to `path`.
    pub fn request_screenshot(&mut self, path: impl Into<String>) {
        self.screenshot_request = Some(path.into());
    }

    /// Take the pending screenshot request, if any.
    pub fn take_screenshot_request(&mut self) -> Option<String> {
        self.screenshot_request.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_bumps_revision_without_overflow() {
        let mut session = Session::new("test");
        assert_eq!(session.revision(), 0);
        session.touch();
        assert_eq!(session.revision(), 1);
    }

    #[test]
    fn setting_a_scene_bumps_the_revision() {
        let mut session = Session::new("test");
        let before = session.revision();
        session.set_scene(Scene::default());
        assert_eq!(session.revision(), before + 1);
    }
}
