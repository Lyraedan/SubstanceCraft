//! The editing session: the mutable state commands act on.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use substancecraft_codecs::ImageF32;
use substancecraft_doc::{BlendMode, Channel, Layer, LayerStack, Mask, Project, Scene, TextureSet, UvTile};
use substancecraft_render::{OrbitCamera, RenderSettings};

/// The maximum number of undo steps kept.
const MAX_UNDO: usize = 64;

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
    /// Undo/redo stacks of texture-set snapshots (the layer data).
    undo_stack: Vec<Vec<TextureSet>>,
    redo_stack: Vec<Vec<TextureSet>>,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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

    /// The active texture set's layer stack.
    pub fn layers(&self) -> Option<&LayerStack> {
        self.project.active_set().map(|set| &set.layers)
    }

    fn snapshot(&self) -> Vec<TextureSet> {
        self.project.texture_sets.clone()
    }

    /// Record the current texture sets for undo and clear the redo stack.
    fn push_undo(&mut self) {
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Whether an undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether a redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Undo the last texture-set change.
    pub fn undo(&mut self) -> bool {
        let Some(sets) = self.undo_stack.pop() else { return false };
        self.redo_stack.push(self.snapshot());
        self.project.texture_sets = sets;
        self.clamp_active_set();
        self.bump_texture_sets();
        true
    }

    /// Redo the last undone change.
    pub fn redo(&mut self) -> bool {
        let Some(sets) = self.redo_stack.pop() else { return false };
        self.undo_stack.push(self.snapshot());
        self.project.texture_sets = sets;
        self.clamp_active_set();
        self.bump_texture_sets();
        true
    }

    fn clamp_active_set(&mut self) {
        if self.project.active_texture_set >= self.project.texture_sets.len() {
            self.project.active_texture_set = 0;
        }
    }

    /// Add a layer on top of the active set's stack.
    pub fn add_layer(&mut self, layer: Layer) {
        self.push_undo();
        if let Some(set) = self.project.active_set_mut() {
            set.layers.push(layer);
        }
        self.bump_texture_sets();
    }

    /// Delete a layer by index from the active set.
    pub fn delete_layer(&mut self, index: usize) -> bool {
        let exists = self.project.active_set().is_some_and(|set| index < set.layers.len());
        if !exists {
            return false;
        }
        self.push_undo();
        if let Some(set) = self.project.active_set_mut() {
            set.layers.remove(index);
        }
        self.bump_texture_sets();
        true
    }

    /// Apply a mutation to one layer. The caller must have validated the index.
    fn edit_layer(&mut self, index: usize, mutate: impl FnOnce(&mut Layer)) {
        if let Some(set) = self.project.active_set_mut()
            && let Some(layer) = set.layers.get_mut(index)
        {
            mutate(layer);
        }
        self.bump_texture_sets();
    }

    fn has_layer(&self, index: usize) -> bool {
        self.project.active_set().is_some_and(|set| index < set.layers.len())
    }

    /// Set a layer's visibility.
    pub fn set_layer_visible(&mut self, index: usize, visible: bool) -> bool {
        if !self.has_layer(index) {
            return false;
        }
        self.push_undo();
        self.edit_layer(index, |layer| layer.visible = visible);
        true
    }

    /// Set a layer's opacity (`0..=1`).
    pub fn set_layer_opacity(&mut self, index: usize, opacity: f32) -> bool {
        if !self.has_layer(index) {
            return false;
        }
        self.push_undo();
        self.edit_layer(index, |layer| layer.opacity = opacity.clamp(0.0, 1.0));
        true
    }

    /// Set a layer's blend mode.
    pub fn set_layer_blend(&mut self, index: usize, blend: BlendMode) -> bool {
        if !self.has_layer(index) {
            return false;
        }
        self.push_undo();
        self.edit_layer(index, |layer| layer.blend = blend);
        true
    }

    /// Rename a layer.
    pub fn set_layer_name(&mut self, index: usize, name: impl Into<String>) -> bool {
        if !self.has_layer(index) {
            return false;
        }
        let name = name.into();
        self.push_undo();
        self.edit_layer(index, |layer| layer.name = name.clone());
        true
    }

    /// Set or clear a layer's mask.
    pub fn set_layer_mask(&mut self, index: usize, mask: Option<Mask>) -> bool {
        if !self.has_layer(index) {
            return false;
        }
        self.push_undo();
        self.edit_layer(index, |layer| layer.mask = mask);
        true
    }

    /// Set a fill layer's colour. Returns false if the layer is missing or not a fill.
    pub fn set_fill_color(&mut self, index: usize, color: [f32; 4]) -> bool {
        if !self.layer_is_fill(index) {
            return false;
        }
        self.push_undo();
        self.edit_layer(index, |layer| {
            if let substancecraft_doc::LayerKind::Fill { color: slot, .. } = &mut layer.kind {
                *slot = color;
            }
        });
        true
    }

    /// Set a fill layer's scalar value. Returns false if the layer is missing or not a fill.
    pub fn set_fill_value(&mut self, index: usize, value: f32) -> bool {
        if !self.layer_is_fill(index) {
            return false;
        }
        self.push_undo();
        self.edit_layer(index, |layer| {
            if let substancecraft_doc::LayerKind::Fill { value: slot, .. } = &mut layer.kind {
                *slot = value;
            }
        });
        true
    }

    fn layer_is_fill(&self, index: usize) -> bool {
        self.project
            .active_set()
            .and_then(|set| set.layers.get(index))
            .is_some_and(|layer| matches!(layer.kind, substancecraft_doc::LayerKind::Fill { .. }))
    }

    /// Move a layer up or down in the stack.
    pub fn move_layer(&mut self, index: usize, up: bool) -> bool {
        let movable = if up { self.project.active_set().is_some_and(|set| index + 1 < set.layers.len()) } else { index > 0 && self.has_layer(index) };
        if !movable {
            return false;
        }
        self.push_undo();
        if let Some(set) = self.project.active_set_mut() {
            if up {
                set.layers.move_up(index);
            } else {
                set.layers.move_down(index);
            }
        }
        self.bump_texture_sets();
        true
    }

    /// Wrap a layer in a new group.
    pub fn group_layer(&mut self, index: usize, name: impl Into<String>) -> bool {
        if !self.has_layer(index) {
            return false;
        }
        self.push_undo();
        if let Some(set) = self.project.active_set_mut() {
            set.layers.group_layer(index, name);
        }
        self.bump_texture_sets();
        true
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
