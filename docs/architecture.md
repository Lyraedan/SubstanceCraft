# Architecture

SubstanceCraft is a Cargo workspace of layered crates plus two applications. Layering is enforced
by `cargo xtask layers`: a crate may depend only on crates at the same or a lower layer, and nothing
below `ui`/`automation` may use egui, winit or native-dialog crates.

## Layers

```
L0 foundation
  substancecraft-geom      meshes (Mesh/Submesh), bounds, UV-derived tangents, BVH ray queries,
                           procedural primitives, glam re-export
  substancecraft-color     sRGB transfer functions, tone mapping (none/Reinhard/ACES/AgX/neutral), exposure
  substancecraft-raster    RasterImage (linear f32 RGBA working surface) with sampling and RGBA8 export
  substancecraft-codecs    image read (PNG/TGA/JPEG/…), EXR read+write, PNG 8/16-bit and TGA write
  substancecraft-tablet    pen input (pressure/tilt); the only crate allowed to use `unsafe` (not wired yet)

L1 document
  substancecraft-doc       project, texture sets, layer stacks, channels, generators/filters, shaders (pure data)
  substancecraft-format    native .sccraft (versioned RON, crash-safe via craft-atomic)

L2 engines
  substancecraft-algo      imaging algorithms (used by material/bake)
  substancecraft-paint     brush engine: dabs, spacing, hardness, pressure, symmetry, jitter;
                           brush/eraser/smudge/clone stamps and UV-island polyfill
  substancecraft-material  Perlin/Simplex/Voronoi/white generators; levels/blur/invert/contrast/brightness filters
  substancecraft-bake      BVH ray bakers: position, world/tangent normal, AO, thickness, curvature, material ID

L3 composition / render
  substancecraft-compose   CPU layer-stack compositor (the oracle), per channel
  substancecraft-gpu       wgpu fill-layer compositor, validated against the CPU oracle
  substancecraft-render    viewport renderer: camera + picking ray, metal/rough PBR, HDRI IBL, composited
                           base-colour texture, grid/wireframe/background, post-effects pass

L4 io
  substancecraft-io        mesh import (glTF 2.0, OBJ), texture export (formats, presets, packing, naming)

L5 engine
  substancecraft-engine    Session + command registry; every user-visible action is a command

L6 ui / automation
  substancecraft-ui        egui/eframe shell (thin, data-driven; renders only on change)
  substancecraft-automation JSON control channel + MCP server (stdio)
```

## The engine/UI seam

The **Session** (`substancecraft-engine::session`) holds the mutable editing state: the open project
(scene, texture sets, layer stacks, shaders), the viewport camera and render settings, the brush and
active tool, the high-poly mesh and last bake, and the file path + dirty flag. It is shared with
background servers through `SharedSession` (`Arc<Mutex<Session>>`); the lock helper recovers from
poisoning so a panic in one command never takes down the editor. Revisions (`scene_revision`,
`texture_set_revision`, `environment_revision`, `bake_revision`) let the viewport and renderer
invalidate caches cheaply.

Every user-visible action is a **command** in the registry: an id, label, menu path, shortcut,
parameter documentation, an optional `enabled` predicate and a `run` closure. The UI, the CLI, the
control channel and the MCP server dispatch commands by id, so behaviour is identical everywhere and
drivable by agents. Commands return `Err`, never panic, for any parameters.

## Document model (overview)

A **project** owns one or more **texture sets**, each of which owns a **layer stack**, a set of
**channels** (base colour, metallic, roughness, normal, height, AO, emissive), UDIM tiles and a
**shader** with parameters. Layers are fill, paint, effect or group nodes; each has visibility,
opacity, a blend mode and an optional mask. Effect layers carry a generator (fills) or filter
(transforms the pixels below). The full model lives in `substancecraft-doc`.

## Rendering pipeline

1. The CPU compositor (`substancecraft-compose`) evaluates a texture set's layer stack into each of
   its channels; the GPU compositor (`substancecraft-gpu`) does the same for flat fill stacks and is
   validated against the CPU oracle.
2. The viewport renderer (`substancecraft-render`) shades the mesh from the composited base colour
   (sampled by UV) with image-based lighting from an environment map, following the PBR model, then
   applies a post-effects pass (bloom + vignette) into the display target egui shows.
3. The layer panel previews the composited base colour; the viewport re-composites and re-renders
   only when a signature of revisions/camera/settings/size changes.

## Project and automation

- **Native format** (`substancecraft-format`): the whole `Project` serialises to versioned RON and is
  written crash-safely through `craft-atomic`; a version header rejects newer files rather than
  misreading them.
- **Automation** (`substancecraft-automation`): a newline-delimited JSON control channel and an MCP
  server (stdio) both expose the command registry; `substancecraft-cli` runs commands, scripts and
  the MCP server headlessly.

## Dependency policy

- Shared, reusable code lives in [craft-libs](https://github.com/storytold/craft-libs), pinned by
  immutable git revision (`craft-ui`, `craft-settings`, `craft-atomic`, `craft-quit`).
- Crates that could be reused in isolation (`codecs`, `format`, `tablet`) avoid workspace dependencies.
- New crates register in `xtask/src/layers.rs` and `[workspace.dependencies]`.
