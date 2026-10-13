# Architecture

SubstanceCraft is a Cargo workspace of layered crates plus two applications. Layering is enforced
by `cargo xtask layers`: a crate may depend only on crates at the same or a lower layer, and nothing
below `ui`/`automation` may use egui, winit or native-dialog crates.

## Layers

```
L0 foundation
  substancecraft-geom      meshes, tangent spaces (MikkTSpace), bounds, BVH, UV tiles, glam re-export
  substancecraft-color     transfer functions (sRGB/linear), tone mapping, exposure, env sampling
  substancecraft-raster    pixel formats 8/16/32f, COW tile buffers, PBR channel math, blend modes
  substancecraft-codecs    PNG/TGA/JPEG/EXR/KTX2 read+write (pure Rust)
  substancecraft-tablet    pen input (pressure/tilt); the only crate allowed to use `unsafe`

L1 document
  substancecraft-doc       project, texture sets, layer stacks, masks, generators, channels (pure data)
  substancecraft-format    native .sccraft (lossless, crash-safe); fails to compile if doc drops a field

L2 engines
  substancecraft-algo      imaging algorithms (blur, levels/HSL, distance, curvature, normal-from-height)
  substancecraft-paint     brush engine: dabs, strokes, spacing, pressure/tilt, jitter, symmetry, projection
  substancecraft-material  procedural noise/generators/filters, smart materials, grunge library
  substancecraft-bake      ray bakers: normal, AO, curvature, position, thickness, WS-normal, ID; cages

L3 composition / render
  substancecraft-compose   CPU layer-stack compositor (the oracle)
  substancecraft-gpu       wgpu layer-stack compositor, paint targets, channel packing
  substancecraft-render    viewport PBR renderer: IBL, display channels, wireframe, post-effects

L4 io
  substancecraft-io        mesh import (glTF 2.0, OBJ), texture import/export, export presets, OpenPBR mapping

L5 engine
  substancecraft-engine    Session + command registry; every user-visible action is a command

L6 ui / automation
  substancecraft-ui        egui/eframe shell (thin, data-driven)
  substancecraft-automation JSON control channel / MCP
```

## The engine/UI seam

The **Session** (`substancecraft-engine::session`) holds the mutable editing state and grows into the
open project, the active texture set, the viewport camera and the undo history. It is shared with
background servers through `SharedSession` (`Arc<Mutex<Session>>`); the lock helper recovers from
poisoning so a panic in one command never takes down the editor.

Every user-visible action is a **command** in the registry: an id, label, menu path, shortcut,
parameter documentation, an optional `enabled` predicate and a `run` closure. The UI, the CLI and the
control channel dispatch commands by id, so behaviour is identical everywhere and drivable by agents.
Commands return `Err`, never panic, for any parameters.

## Document model (overview)

A **project** owns one or more **texture sets**, each of which owns a **layer stack** and a set of
**channels** (base colour, roughness, metalness, normal, height, AO, …). Layers are fill, paint,
effect or group nodes; each may carry a mask. Baking produces **mesh maps** that feed generators and
masks. The full model lives in `substancecraft-doc`; see that crate as the milestones land.

## Rendering pipeline (target shape)

1. The GPU compositor (`substancecraft-gpu`) evaluates each texture set's layer stack into its
   channels, parallel with the CPU oracle (`substancecraft-compose`) used for tests and headless export.
2. The viewport renderer (`substancecraft-render`) shades the mesh from the composited channels with
   image-based lighting from an environment map, following OpenPBR.
3. Painting edits the active layer's channel targets; the compositor re-evaluates on `Session::revision`.

## Dependency policy

- Shared, reusable code lives in [craft-libs](https://github.com/storytold/craft-libs), pinned by
  immutable git revision.
- Crates that could be reused in isolation (`codecs`, `format`, `tablet`) avoid workspace deps.
- New crates register in `xtask/src/layers.rs` and `[workspace.dependencies]`.
