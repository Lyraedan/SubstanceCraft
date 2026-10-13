//! The editing session: the mutable state commands act on.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

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
    /// The viewport camera.
    camera: OrbitCamera,
    /// The viewport render settings (display mode, exposure, tone mapping).
    settings: RenderSettings,
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
            camera: OrbitCamera::default(),
            settings: RenderSettings::default(),
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
