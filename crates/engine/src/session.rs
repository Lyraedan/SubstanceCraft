//! The editing session: the mutable state commands act on.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use substancecraft_codecs::ImageF32;
use substancecraft_doc::{Channel, Project, Scene, TextureSet, UvTile};
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
    /// The open project: the scene and its texture sets.
    project: Project,
    /// Bumped only when the scene changes, so the renderer re-uploads geometry.
    scene_revision: u64,
    /// Bumped when the texture sets change (split, select, rename, channels, tiles).
    texture_set_revision: u64,
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
            project: Project::new("untitled"),
            scene_revision: 1,
            texture_set_revision: 1,
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
        &self.project.scene
    }

    /// Replace the scene, rebuilding the texture sets, and bump the revisions.
    pub fn set_scene(&mut self, scene: Scene) {
        self.project.scene = scene;
        self.project.rebuild_texture_sets();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.bump_texture_sets();
    }

    /// Bumped when the scene changes, so the renderer re-uploads geometry.
    pub fn scene_revision(&self) -> u64 {
        self.scene_revision
    }

    /// The open project.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The texture sets.
    pub fn texture_sets(&self) -> &[TextureSet] {
        &self.project.texture_sets
    }

    /// The index of the active texture set.
    pub fn active_texture_set(&self) -> usize {
        self.project.active_texture_set
    }

    /// The material ids of the active texture set, for viewport isolation.
    pub fn active_materials(&self) -> Vec<u32> {
        self.project.active_materials()
    }

    /// Bumped when the texture sets change, so the viewport re-applies selection.
    pub fn texture_set_revision(&self) -> u64 {
        self.texture_set_revision
    }

    fn bump_texture_sets(&mut self) {
        self.texture_set_revision = self.texture_set_revision.saturating_add(1);
        self.touch();
    }

    /// Select a texture set; returns false if out of range.
    pub fn select_texture_set(&mut self, index: usize) -> bool {
        if self.project.select(index) {
            self.bump_texture_sets();
            true
        } else {
            false
        }
    }

    /// Rename the active texture set.
    pub fn rename_texture_set(&mut self, name: impl Into<String>) {
        if let Some(set) = self.project.active_set_mut() {
            set.name = name.into();
            self.bump_texture_sets();
        }
    }

    /// Set the active texture set's resolution (clamped to a sane range).
    pub fn set_texture_resolution(&mut self, resolution: u32) {
        if let Some(set) = self.project.active_set_mut() {
            set.resolution = resolution.clamp(32, 8192);
            self.bump_texture_sets();
        }
    }

    /// Enable or disable a channel on the active texture set.
    pub fn set_texture_channel(&mut self, channel: Channel, enabled: bool) {
        if let Some(set) = self.project.active_set_mut() {
            set.set_channel(channel, enabled);
            self.bump_texture_sets();
        }
    }

    /// Add a UDIM tile to the active texture set (no-op if present).
    pub fn add_uv_tile(&mut self, tile: UvTile) {
        if let Some(set) = self.project.active_set_mut()
            && !set.uv_tiles.contains(&tile)
        {
            set.uv_tiles.push(tile);
            set.uv_tiles.sort();
            self.bump_texture_sets();
        }
    }

    /// Remove a UDIM tile from the active texture set; at least one tile always remains.
    pub fn remove_uv_tile(&mut self, tile: UvTile) {
        if let Some(set) = self.project.active_set_mut()
            && set.uv_tiles.len() > 1
            && set.uv_tiles.contains(&tile)
        {
            set.uv_tiles.retain(|existing| *existing != tile);
            self.bump_texture_sets();
        }
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

    #[test]
    fn a_new_session_has_one_texture_set() {
        let session = Session::new("test");
        assert_eq!(session.texture_sets().len(), 1);
        assert_eq!(session.active_texture_set(), 0);
    }

    #[test]
    fn selecting_out_of_range_fails_without_panicking() {
        let mut session = Session::new("test");
        assert!(!session.select_texture_set(99));
        assert_eq!(session.active_texture_set(), 0);
    }

    #[test]
    fn resolution_is_clamped() {
        let mut session = Session::new("test");
        session.set_texture_resolution(8);
        assert_eq!(session.texture_sets()[0].resolution, 32);
        session.set_texture_resolution(999_999);
        assert_eq!(session.texture_sets()[0].resolution, 8192);
    }

    #[test]
    fn channel_and_uv_tile_edits_bump_revision() {
        let mut session = Session::new("test");
        let before = session.texture_set_revision();
        session.set_texture_channel(Channel::Emissive, true);
        assert!(session.texture_sets()[0].has_channel(Channel::Emissive));
        session.add_uv_tile(UvTile { u: 1, v: 0 });
        assert!(session.texture_sets()[0].uv_tiles.contains(&UvTile { u: 1, v: 0 }));
        assert!(session.texture_set_revision() > before);
    }

    #[test]
    fn the_last_uv_tile_cannot_be_removed() {
        let mut session = Session::new("test");
        session.remove_uv_tile(UvTile::DEFAULT);
        assert_eq!(session.texture_sets()[0].uv_tiles.len(), 1);
    }
}
