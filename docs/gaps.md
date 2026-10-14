# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-15 (M12 complete)._

## Workflow & UX depth (newly identified)

15. **Layer blending and fills are shallow.** Only 8 blend modes (Painter has ~32); masks are
    constant coverage only (no painted, image or geometry masks); fill layers are constant-only —
    no image or mesh-map source, no UV/tri-planar/planar projection, no tiling X/Y and no
    per-channel strength.
16. **No parts workflow.** Cannot select a connected piece, UV island, imported object or material
    ID and fill it, paint only there, mask a layer or add a generator; no isolate (`I`) / hide (`H`)
    or right-click context menu on the model.
17. **Painting lacks depth.** No brush alpha tips, lazy mouse, straight-line (Shift) strokes,
    stencils, triangle/element polyfill modes, mask eraser, eyedropper or saved brush presets.
18. **No content library.** No material presets, smart materials, smart masks, HDRI/environment
    library, textures shelf, shelf search/filter, drag-and-drop or on-disk user library.
19. **Import/export interop gaps.** No FBX import; no "import a texture set as one material" (name
    matching, gloss→roughness, DirectX→OpenGL normal); no UV padding, height-into-normal export or
    DirectX-normal templates.
20. **Views and onboarding.** No 2D UV view of channels/baked maps, no AO/height/matcap display
    modes, height is not shown as bumps and AO does not occlude ambient; no welcome/quick-start/
    recent-files, preferences dialog, keyboard-shortcuts dialog or history click-to-jump panel.
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
