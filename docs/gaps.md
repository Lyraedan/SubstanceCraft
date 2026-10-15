# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-15 (M12 complete)._

## Workflow & UX depth (newly identified)

15. **Layer masks lack painted/geometry support.** Blending (26 modes + per-channel opacity), fill
    sources (uniform / image / procedural / baked mesh map with UV / planar / tri-planar projection,
    tiling and per-channel strength) and mask-effect stacks (fill, generator, filter, mesh map,
    image) are in; masks are not yet paint-brushable and there are no geometry/part masks.
16. **Parts workflow is mostly in.** Connected-piece restriction, UV-island/material catalogues
    (`scene.parts {mode}`), picking (`scene.pick_part`), viewport isolate/hide (`I` / `H` / `Esc`)
    and part fill/mask commands (`layer.add_part_fill`, `layer.mask_to_part`) are in; still missing
    imported-object catalogues, a hover highlight and a right-click context menu (keyboard only today).
17. **Painting is deeper now.** Brush alpha tips, lazy mouse, straight-line (Shift) strokes, an
    eyedropper and saved brush presets are in; still missing stencils, triangle/element polyfill
    modes, a mask eraser and native pen pressure/tilt.
18. **Content library is partial.** Material presets, smart materials and smart masks are in
    (built-in plus in-session user-saved, with shelf tiles and search/filter); still missing a
    built-in HDRI/environment library, a textures shelf, drag-and-drop and on-disk persistence of
    the user library.
19. **Import/export interop.** Model import (OBJ/glTF/STL/PLY/FBX/Collada), texture-set import (name
    matching, gloss→roughness, DirectX→OpenGL normal), height-into-normal baking and DirectX-normal
    export are in; still missing UV padding, per-channel sRGB assignment, batch export across texture
    sets, saved export presets and USD/OpenPBR material export.
20. **Views are in; onboarding is not.** A 2D UV view (with UV wireframe), AO/height/matcap display
    modes, a keyboard-shortcuts dialog and a history panel with multi-step jump are in; still missing
    a welcome/quick-start screen, recent files, a preferences dialog, height-as-bumps and AO
    occluding ambient.
21. **Web build.** A wasm/WebGPU web build is a candidate target alongside the native app.

## Release (deferred)

1. **No packaging/signing.** `cargo xtask build` copies the raw binaries into `Build/`; a signed
   macOS `.app`, Windows installer and Linux tarball are not produced.
2. **A single perf baseline** is recorded on the dev machine; budgets are not enforced in CI yet
   (`enforce = false`).

## Automation polish (deferred from M11)

1. **No typed Rust scripting layer** over the command API (only the JSON/Script/MCP surfaces).
2. **MCP exposes tools only** — no resources or prompts.
3. **The control channel does not stream events** (only request/response).

## Project-format polish

6. **RON text is large** (paint images); no binary container. No templates/New Project window or
   startup recovery prompt; environment/high-poly/baked maps are not saved.

## Workflow-depth / export / baking / painting polish

7. **One shading model**; specular parameter unused; no anchors/instancing/physical size; cheap post-effects.
8. **Per-tile UDIM content**; no USD/OpenPBR material export, saved presets, batch export, edge padding.
9. **No cage mesh/skew**; per-map settings; baked maps not wired into generators; CPU baker.
10. **No grunge library**; effect parameters are editable (layer panel + `layer.set_effect`).
11. **No native pen pressure/tilt; no physics brushes; CPU painting; not UDIM-aware.**
12. **GPU compositor limited to flat fill stacks; undo snapshots the whole set list.**
13. **Grid alignment; camera persistence; prefiltered IBL; selection outline.**

## Documentation

14. Parity matrices beyond the tracked dimensions are still partial.
