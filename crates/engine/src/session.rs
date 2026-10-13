//! The editing session: the mutable state commands act on.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use substancecraft_bake::{BakeMap, BakeSettings, Baker};
use substancecraft_codecs::ImageF32;
use substancecraft_doc::{BlendMode, Channel, Layer, LayerKind, LayerStack, Mask, Project, Scene, Shader, ShaderParams, TextureSet, UvTile};
use substancecraft_paint::{BrushSettings, StrokeState, ToolKind, clone_stamp, erase_stamp, paint_stamp, smudge_stamp};
use substancecraft_raster::RasterImage;
use substancecraft_render::{OrbitCamera, RenderSettings};

use crate::workspace::{Panel, RightTab, Workspace};

/// The maximum number of undo steps kept.
const MAX_UNDO: usize = 64;

/// The largest working paint resolution (the texture set resolution may be higher; painting and
/// the live viewport composite use this cap for performance).
const MAX_PAINT_RESOLUTION: u32 = 1024;

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
    /// The brush parameters.
    brush: BrushSettings,
    /// The in-progress stroke, if the pointer is down.
    stroke: Option<StrokeState>,
    /// The texel-per-screen-pixel scale for the current stroke (0 = texture-space sizing).
    stroke_scale: f32,
    /// The pre-stroke image snapshot used by clone and smudge.
    stroke_source: Option<RasterImage>,
    /// The active tool.
    tool: ToolKind,
    /// The selected layer index in the active set.
    active_layer: Option<usize>,
    /// An imported high-poly mesh for baking tangent-space normals.
    high_poly: Option<Arc<substancecraft_geom::Mesh>>,
    /// Bumped when the high-poly changes.
    high_poly_revision: u64,
    /// Bake parameters.
    bake_settings: BakeSettings,
    /// The most recent baked maps.
    baked: Vec<(BakeMap, RasterImage)>,
    /// A log of the last bake.
    bake_log: Vec<String>,
    /// Bumped when baked maps change.
    bake_revision: u64,
    /// The path the project was last saved to or opened from.
    file_path: Option<PathBuf>,
    /// Whether there are unsaved document changes.
    dirty: bool,
    /// Workspace layout view state (panel visibility and dock selection).
    workspace: Workspace,
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
            brush: BrushSettings::default(),
            stroke: None,
            stroke_scale: 0.0,
            stroke_source: None,
            tool: ToolKind::default(),
            active_layer: None,
            high_poly: None,
            high_poly_revision: 1,
            bake_settings: BakeSettings::default(),
            baked: Vec::new(),
            bake_log: Vec::new(),
            bake_revision: 1,
            file_path: None,
            dirty: false,
            workspace: Workspace::default(),
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
        self.dirty = true;
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

    /// Set the active texture set's shading model.
    pub fn set_shader(&mut self, shader: Shader) {
        if let Some(set) = self.project.active_set_mut() {
            set.shader = shader;
            self.bump_texture_sets();
        }
    }

    /// Replace the active texture set's shader parameters.
    pub fn set_shader_params(&mut self, params: ShaderParams) {
        if let Some(set) = self.project.active_set_mut() {
            set.shader_params = params;
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

    /// The brush parameters.
    pub fn brush(&self) -> &BrushSettings {
        &self.brush
    }

    /// The brush parameters, mutably.
    pub fn brush_mut(&mut self) -> &mut BrushSettings {
        &mut self.brush
    }

    /// The active tool.
    pub fn tool(&self) -> ToolKind {
        self.tool
    }

    /// Select the active tool.
    pub fn set_tool(&mut self, tool: ToolKind) {
        self.tool = tool;
    }

    /// The selected layer index in the active texture set, if any.
    pub fn active_layer_index(&self) -> Option<usize> {
        self.active_layer
    }

    /// Select a layer (or clear the selection).
    pub fn set_active_layer(&mut self, index: Option<usize>) {
        self.active_layer = index;
    }

    /// The working paint resolution for the active set (capped).
    pub fn paint_resolution(&self) -> u32 {
        self.project.active_set().map(|set| set.resolution.clamp(64, MAX_PAINT_RESOLUTION)).unwrap_or(256)
    }

    /// The paint layer that strokes target: the active layer if it is a paint layer, else the
    /// topmost paint layer.
    fn paint_target_index(&self) -> Option<usize> {
        let set = self.project.active_set()?;
        if let Some(index) = self.active_layer
            && set.layers.get(index).is_some_and(|layer| matches!(layer.kind, LayerKind::Paint { .. }))
        {
            return Some(index);
        }
        set.layers.layers.iter().rposition(|layer| matches!(layer.kind, LayerKind::Paint { .. }))
    }

    /// Ensure a paint layer with an image at `resolution` exists and is the target.
    fn ensure_paint_target(&mut self, resolution: u32) {
        let target = self.paint_target_index();
        let Some(set) = self.project.active_set_mut() else { return };
        let index = match target {
            Some(index) => index,
            None => {
                set.layers.push(Layer::paint("Paint"));
                set.layers.len() - 1
            }
        };
        if let Some(layer) = set.layers.get_mut(index)
            && let LayerKind::Paint { image } = &mut layer.kind
        {
            let needs = image.as_ref().is_none_or(|img| img.width != resolution || img.height != resolution);
            if needs {
                *image = Some(RasterImage::filled(resolution, resolution, [0.0, 0.0, 0.0, 0.0]));
            }
        }
        self.active_layer = Some(index);
    }

    /// The active paint target image, mutably.
    fn active_paint_image_mut(&mut self) -> Option<&mut RasterImage> {
        let index = self.paint_target_index()?;
        let set = self.project.active_set_mut()?;
        match &mut set.layers.get_mut(index)?.kind {
            LayerKind::Paint { image: Some(image) } => Some(image),
            _ => None,
        }
    }

    /// A snapshot of the active paint target image (for clone/smudge sources).
    fn active_paint_image_snapshot(&self) -> Option<RasterImage> {
        let index = self.paint_target_index()?;
        let set = self.project.active_set()?;
        match &set.layers.get(index)?.kind {
            LayerKind::Paint { image: Some(image) } => Some(image.clone()),
            _ => None,
        }
    }

    /// Begin a paint stroke at UV `(u, v)`. Ensures a paint target exists and records one undo step.
    /// `scale` is the number of texture texels per screen pixel (0 disables screen-space sizing).
    pub fn begin_stroke(&mut self, u: f32, v: f32, pressure: f32, scale: f32) {
        if self.project.active_set().is_none() {
            return;
        }
        self.push_undo();
        let resolution = self.paint_resolution();
        self.ensure_paint_target(resolution);
        let tool = self.tool;
        self.stroke_source = if matches!(tool, ToolKind::Clone | ToolKind::Smudge) { self.active_paint_image_snapshot() } else { None };
        self.stroke_scale = scale;
        self.stroke = Some(StrokeState::default());
        self.stamp_stroke(u, v, pressure);
    }

    /// Continue the current paint stroke to UV `(u, v)`.
    pub fn extend_stroke(&mut self, u: f32, v: f32, pressure: f32) {
        if self.stroke.is_none() {
            return;
        }
        self.stamp_stroke(u, v, pressure);
    }

    /// End the current paint stroke.
    pub fn end_stroke(&mut self) {
        self.stroke = None;
        self.stroke_source = None;
    }

    /// Whether a stroke is in progress.
    pub fn is_stroking(&self) -> bool {
        self.stroke.is_some()
    }

    /// Place the dabs for one stroke point and stamp the active tool.
    fn stamp_stroke(&mut self, u: f32, v: f32, pressure: f32) {
        let brush = self.brush;
        let scale = self.stroke_scale;
        let tool = self.tool;
        let radius = brush.radius_at(pressure, scale);
        let alpha = brush.alpha_at(pressure);
        let step = (brush.spacing.clamp(0.01, 1.0) * radius).max(0.5);

        let mut stroke = self.stroke.take().unwrap_or_default();
        let mut positions = Vec::new();
        if let Some(image) = self.active_paint_image_mut() {
            let center = uv_to_pixels(u, v, image.width, image.height);
            stroke.dabs(center, step, &mut positions);
        }
        self.stroke = Some(stroke);

        let source = self.stroke_source.take();
        if let Some(image) = self.active_paint_image_mut() {
            let (width, height) = (image.width as f32, image.height as f32);
            for position in positions {
                for center in brush.symmetry.centers(position, width, height) {
                    match tool {
                        ToolKind::Brush => paint_stamp(image, center, radius, brush.hardness, brush.color, alpha),
                        ToolKind::Eraser => erase_stamp(image, center, radius, brush.hardness, alpha),
                        ToolKind::Clone => {
                            if let Some(source) = &source {
                                clone_stamp(image, source, center, radius, brush.hardness, brush.clone_offset, alpha);
                            }
                        }
                        ToolKind::Smudge => {
                            if let Some(source) = &source {
                                smudge_stamp(image, source, center, position, radius, brush.hardness, brush.smudge);
                            }
                        }
                        ToolKind::Polyfill => {}
                    }
                }
            }
        }
        self.stroke_source = source;
        self.bump_texture_sets();
    }

    /// Fill the UV island containing `triangle` on the active paint target.
    pub fn polyfill(&mut self, triangle: u32) {
        if self.project.active_set().is_none() {
            return;
        }
        self.push_undo();
        let resolution = self.paint_resolution();
        self.ensure_paint_target(resolution);
        let color = self.brush.color;
        let mesh = self.project.scene.mesh.clone();
        if let Some(image) = self.active_paint_image_mut() {
            let _ = substancecraft_paint::polyfill(&mesh, image, triangle, color);
        }
        self.bump_texture_sets();
    }

    /// The imported high-poly mesh, if any.
    pub fn high_poly(&self) -> Option<&Arc<substancecraft_geom::Mesh>> {
        self.high_poly.as_ref()
    }

    /// Set (or clear) the high-poly mesh used for tangent-space normal baking.
    pub fn set_high_poly(&mut self, mesh: Option<substancecraft_geom::Mesh>) {
        self.high_poly = mesh.map(Arc::new);
        self.high_poly_revision = self.high_poly_revision.saturating_add(1);
        self.touch();
    }

    /// Bumped when the high-poly changes.
    pub fn high_poly_revision(&self) -> u64 {
        self.high_poly_revision
    }

    /// Bake parameters.
    pub fn bake_settings(&self) -> &BakeSettings {
        &self.bake_settings
    }

    /// Bake parameters, mutably.
    pub fn bake_settings_mut(&mut self) -> &mut BakeSettings {
        &mut self.bake_settings
    }

    /// The most recent baked maps.
    pub fn baked_maps(&self) -> &[(BakeMap, RasterImage)] {
        &self.baked
    }

    /// The last bake log.
    pub fn bake_log(&self) -> &[String] {
        &self.bake_log
    }

    /// Bumped when baked maps change.
    pub fn bake_revision(&self) -> u64 {
        self.bake_revision
    }

    /// The baked image for a map, if present.
    pub fn baked_map(&self, map: BakeMap) -> Option<&RasterImage> {
        self.baked.iter().find(|(kind, _)| *kind == map).map(|(_, image)| image)
    }

    /// Bake every mesh map for the current scene, replacing any previous results.
    pub fn run_bake(&mut self) {
        let started = std::time::Instant::now();
        let settings = self.bake_settings;
        let results: Vec<(BakeMap, RasterImage)> = {
            let baker = Baker::new(&self.project.scene.mesh, self.high_poly.as_deref());
            BakeMap::ALL.iter().map(|&map| (map, baker.bake(map, &settings))).collect()
        };
        let has_high_poly = self.high_poly.is_some();

        self.bake_log.clear();
        self.bake_log.push(format!("baked {} maps at {}²", results.len(), settings.resolution));
        if !has_high_poly {
            self.bake_log.push("no high-poly imported: tangent-space normal is flat".to_string());
        }
        self.bake_log.push(format!("took {:.0} ms", started.elapsed().as_secs_f64() * 1000.0));
        self.baked = results;
        self.bake_revision = self.bake_revision.saturating_add(1);
        self.set_status(format!("baked {} maps", self.baked.len()));
        self.touch();
    }

    /// Save a baked map to a PNG. Returns a message on failure.
    pub fn save_baked(&self, map: BakeMap, path: &str) -> Result<(), String> {
        let image = self.baked_map(map).ok_or_else(|| format!("no baked {} map", map.id()))?;
        substancecraft_codecs::save_png(path, image.width, image.height, &image.to_rgba8_linear()).map_err(|err| err.to_string())
    }

    /// The path the project was last saved to or opened from.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Whether there are unsaved document changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the document dirty (autosave saves without clearing the flag).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// The workspace layout view state.
    pub fn workspace(&self) -> Workspace {
        self.workspace
    }

    /// Replace the workspace layout view state.
    pub fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }

    /// Show or hide a panel.
    pub fn set_panel(&mut self, panel: Panel, visible: bool) {
        self.workspace.set_visible(panel, visible);
    }

    /// Toggle a panel; returns its new visibility.
    pub fn toggle_panel(&mut self, panel: Panel) -> bool {
        let visible = !self.workspace.is_visible(panel);
        self.workspace.set_visible(panel, visible);
        visible
    }

    /// Select the right-middle dock tab.
    pub fn set_right_tab(&mut self, tab: RightTab) {
        self.workspace.right_tab = tab;
    }

    /// An estimate of the resident pixel memory (paint images + baked maps), in bytes.
    pub fn cache_bytes(&self) -> u64 {
        let paint: u64 = self.project.texture_sets.iter().map(|set| layer_bytes(&set.layers)).sum();
        let baked: u64 = self.baked.iter().map(|(_, image)| u64::from(image.width) * u64::from(image.height) * 16).sum();
        paint.saturating_add(baked)
    }

    /// Write a copy of the project to `path` without changing the current file path or dirty flag.
    pub fn write_copy(&self, path: &Path) -> Result<(), String> {
        substancecraft_format::save(&self.project, path).map_err(|err| err.to_string())
    }

    /// The window title: filename with a leading `*` when there are unsaved changes.
    pub fn project_title(&self) -> String {
        let name = self.file_path.as_ref().and_then(|path| path.file_name()).and_then(|name| name.to_str()).unwrap_or("untitled");
        if self.dirty { format!("* {name}") } else { name.to_string() }
    }

    /// Save to `path` (or the current file when `None`).
    pub fn save(&mut self, path: Option<PathBuf>) -> Result<(), String> {
        let target = match path {
            Some(path) => path,
            None => self.file_path.clone().ok_or_else(|| "no file path; use Save As".to_string())?,
        };
        substancecraft_format::save(&self.project, &target).map_err(|err| err.to_string())?;
        self.file_path = Some(target.clone());
        self.dirty = false;
        self.set_status(format!("saved {}", target.display()));
        Ok(())
    }

    /// Open a project from `path`, replacing the current one.
    pub fn open(&mut self, path: &Path) -> Result<(), String> {
        let project = substancecraft_format::load(path).map_err(|err| err.to_string())?;
        self.project = project;
        self.high_poly = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.baked.clear();
        self.bake_log.clear();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.bump_texture_sets();
        self.file_path = Some(path.to_path_buf());
        self.dirty = false;
        self.set_status(format!("opened {}", path.display()));
        Ok(())
    }

    /// Start a new, empty project.
    pub fn new_project(&mut self) {
        self.project = Project::new("untitled");
        self.high_poly = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.baked.clear();
        self.bake_log.clear();
        self.scene_revision = self.scene_revision.saturating_add(1);
        self.bump_texture_sets();
        self.file_path = None;
        self.dirty = false;
        self.set_status("new project");
    }
}

/// Convert UV to texture pixel coordinates, wrapping into `0..1`.
fn uv_to_pixels(u: f32, v: f32, width: u32, height: u32) -> [f32; 2] {
    [u.rem_euclid(1.0) * width as f32, v.rem_euclid(1.0) * height as f32]
}

/// The resident byte size of the paint images in a layer stack (recursing into groups).
fn layer_bytes(stack: &LayerStack) -> u64 {
    let mut total = 0u64;
    for layer in &stack.layers {
        match &layer.kind {
            LayerKind::Paint { image: Some(image) } => {
                total = total.saturating_add(u64::from(image.width).saturating_mul(u64::from(image.height)).saturating_mul(16));
            }
            LayerKind::Group { children } => total = total.saturating_add(layer_bytes(children)),
            _ => {}
        }
    }
    total
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

    #[test]
    fn workspace_panels_toggle_and_tabs_select() {
        let mut session = Session::new("test");
        assert!(session.workspace().is_visible(Panel::Assets));
        assert!(!session.toggle_panel(Panel::Assets));
        assert!(!session.workspace().is_visible(Panel::Assets));
        session.set_panel(Panel::Assets, true);
        assert!(session.workspace().is_visible(Panel::Assets));
        session.set_right_tab(RightTab::Settings);
        assert_eq!(session.workspace().right_tab, RightTab::Settings);
    }

    #[test]
    fn cache_bytes_counts_paint_images() {
        let mut session = Session::new("test");
        assert_eq!(session.cache_bytes(), 0);
        session.begin_stroke(0.5, 0.5, 1.0, 0.0);
        session.end_stroke();
        assert!(session.cache_bytes() > 0);
    }
}
