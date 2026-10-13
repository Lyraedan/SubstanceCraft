# Target-app parity

Where SubstanceCraft stands against Adobe Substance 3D Painter, dimension by dimension. Status
lines carry the date; update the dimension when your work moves it. SubstanceCraft is a clean-room
reimplementation: parity means comparable behaviour and results, never copied code or assets.

_Last reviewed: 2026-10-13 (M11 complete)._

| Dimension | Weight | Status | Notes |
|---|---|---|---|
| Project & texture-set workflow | high | 40% | Split by material, per-set channels/resolution/UDIM, solo, save/open (M2, M10). |
| Layer stack & blending | high | 45% | Fill/group layers, masks, 8 blend modes, opacity, visibility, undo/redo; CPU + GPU compositor (M3). |
| Painting engine | high | 62% | Brush/eraser/smudge/clone/polyfill, screen-space sizing, symmetry, jitter, active-layer target, live composited result (M4–M5). |
| Procedural content (generators/filters/smart materials) | high | 40% | Generators, filters, effect layers, shelf (M6). |
| Baking (mesh maps + cage) | high | 50% | BVH-accelerated bakers, high-poly import, save to PNG (M7). |
| Export (presets, channels, UDIM, OpenPBR) | high | 55% | Channel export (PNG 8/16, TGA, EXR), presets, packing, naming, UDIM, round-trip (M8). |
| Viewport & PBR preview | medium | 90% | Import, camera, PBR, IBL, tone mapping, display modes, grid, wireframe, composited base colour, post-effects, snapshot. |
| Shaders (OpenPBR / ASM) | medium | 35% | Per-set shader selection, editable params, post-effects (M9). |
| Automation & scripting | medium | 70% | 85 commands; JSON control channel, **MCP server**, headless CLI script runner, introspection (M0–M11). |
| Native project format | medium | 55% | `.sccraft` (RON via `craft-atomic`), versioned, save/open/save-as/new, title + dirty, autosave/recovery (M10). |
| Performance | medium | n/a | No measurements yet. |

## Automation (M11) — complete

- [x] Every user-visible action is a registered command (85 commands with id/label/menu/params).
- [x] JSON control channel (`system.ping`, `system.list_commands`, `system.describe`, dispatch) shared with the live app session.
- [x] Introspection commands: `session.info`, `scene.info`, `viewport.state`, `app.capabilities`.
- [x] **MCP server** over stdio: `initialize`, `tools/list`, `tools/call` (id `.` → `_`).
- [x] Headless CLI: `commands`, `run`, `script <file>`, `serve`, `mcp`, `info`.
- [ ] A typed Rust scripting layer over the command API.
- [ ] MCP resources/prompts (tools only today).

## Project lifecycle (M10) — complete

- [x] Serialise the whole `Project` (scene, texture sets, layer stacks, shader, paint images) to `.sccraft`.
- [x] Crash-safe atomic writes (`craft-atomic`) with a version header.
- [x] `file.new` / `file.open` / `file.save` / `file.save_as` / `file.info`; window title + dirty state.
- [x] Autosave every 60 s while dirty, and `file.recover`.
- [ ] Binary container for paint images (RON text is large).
- [ ] Project templates and a New Project window.
- [ ] A crash-recovery prompt on startup; environment/high-poly/baked maps are not saved.

## Workflow depth (M9) — partial

- [x] Per-texture-set shader selection (OpenPBR / Metal-Rough / Spec-Gloss).
- [x] Editable shader parameters (metallic, roughness, specular, base-colour tint) feeding viewport shading.
- [x] Post-effects: vignette and bloom as a post-process pass.
- [ ] Distinct OpenPBR vs spec/gloss shading models (the specular parameter is not yet consumed).
- [ ] Anchor points and layer instancing across texture sets.
- [ ] Physical-size scaling for UV projection.

## Export (M8) — complete

- [x] Channel export at a chosen resolution: PNG 8-bit, PNG 16-bit, TGA, EXR 32-bit float.
- [x] Presets: OpenPBR (all channels), glTF (metallic-roughness + occlusion packing), Unreal (ORM), Unity (mask), Spec/Gloss.
- [x] Channel packing (R/G/B/A from any channel) and naming templates (`$textureSet`, `$channel`, `$udim`).
- [x] UDIM naming (`$udim`) wired; one file per UV tile.
- [x] Round-trip test: exported PNGs re-import to identical values.
- [ ] Per-tile UDIM content (painting is not UDIM-aware yet).
- [ ] USD/OpenPBR material export; batch export across texture sets; saved preset/config files.

## Baking (M7) — complete

- [x] BVH-accelerated ray queries (`geom::Bvh`) with a brute-force cross-check test.
- [x] Bakers: position, world-space normal, tangent-space normal (low→high), ambient occlusion, thickness, curvature, material ID.
- [x] High-poly mesh import; per-texel UV rasterisation with interpolated attributes.
- [x] Bake panel (resolution, AO samples, high-poly, run, per-map save) and a bake log; `bake.save` writes PNGs.
- [ ] Explicit/implicit cage mesh export; skew correction; edge padding; per-map custom settings.
- [ ] Bake to the texture-set channel images used by generators/masks.
- [ ] GPU baker (CPU today).

## Procedural content (M6) — complete

- [x] Generators: Perlin (fBm), Simplex (fBm), Voronoi (Worley F1), White noise (seeded, deterministic).
- [x] Filters: Levels, Blur (separable box), Invert, Contrast, Brightness.
- [x] Effect layers: a generator fills, a filter transforms the pixels below; opacity blends the filter.
- [x] Shelf panel listing generators and filters with one-click add; live composite preview.
- [ ] Smart materials / masks (saved layer-stack snippets).
- [ ] Grunge/curvature/AO-driven grunge; per-filter parameters UI beyond defaults.
- [ ] GPU compositing of effect layers (CPU path today).

## Painting tools (M5) — complete

- [x] Tools: brush, eraser, smudge, clone, polyfill (UV-island fill).
- [x] Screen-space brush sizing (texels-per-screen-pixel from the hit triangle's UV density).
- [x] Active-layer target (`layer.select`) plus paint-target fallback.
- [x] Jitter (particle-like scatter) and clone offset.
- [x] Tool palette and per-tool brush controls in the UI.
- [ ] Native pen pressure/tilt (`substancecraft-tablet`).
- [ ] Physics brushes; streamed/GPU painting; BVH raycast.

## Painting (M4) — complete

- [x] Brush engine: dabs, spacing, hardness falloff, size/opacity pressure, symmetry, colour.
- [x] Projection painting: CPU raycast against the mesh, barycentric UV interpolation, paint into the active layer's texture target.
- [x] Paint layer target management; a stroke is one undo step.
- [x] Composited base colour fed back into the 3D viewport (sampled by UV).
- [x] Painterly controls: eraser-ish fill via layers, brush presets/settings, symmetry, colour picker.
- [ ] Native pen pressure/tilt (`substancecraft-tablet`) — mouse pressure is 1.0 for now.
- [ ] Screen-space brush sizing (radius is in texture pixels).
- [ ] GPU/streamed painting; per-layer paint blending modes for scalars.

## Layer stack & blending (M3) — complete

- [x] Document model: fill / paint / effect / group layer kinds, per-layer mask + opacity + visibility + blend mode.
- [x] 8 blend modes (normal, multiply, screen, overlay, add, subtract, darken, lighten).
- [x] CPU compositor oracle (per channel), fully unit-tested.
- [x] GPU compositor for flat fill stacks, validated against an independent CPU reference.
- [x] Layer-stack UI: add/delete/reorder/group, visibility, opacity, blend, fill colour/value, mask, and a live composite preview.
- [x] Undo/redo through commands (snapshot-based).
- [ ] GPU compositor for groups/effects/painted layers.
- [ ] Composited channels driving the 3D viewport (lands with painting, M4).

## Texture sets & UV tiles (M2) — complete

- [x] Split the imported mesh into texture sets by material id.
- [x] Texture-set list UI with selection, rename, triangle counts.
- [x] Per-set resolution (512–4096).
- [x] Per-set channels (OpenPBR channel family) with toggles.
- [x] UDIM / UV-tile list derived from the mesh UVs, with add/remove.
- [x] Solo (isolate the active set in the viewport).
- [ ] Per-set shader instances and material reassignment.
- [ ] Cross-tile UDIM painting (needs the painting engine, M4).

## Viewport / PBR preview (M1) — complete

- [x] App window, theme, command registry, control channel (M0).
- [x] Mesh import (glTF, OBJ) with tangents.
- [x] Orbit/pan/dolly/focus camera; perspective and orthographic.
- [x] Metal/rough PBR shading.
- [x] Image-based lighting from an HDRI (`.hdr`/`.exr`) with a hemisphere fallback.
- [x] Tone mapping and exposure.
- [x] Display modes (base colour, normal, roughness, metalness, UV checker, wireframe).
- [x] Ground grid and sky/environment background.
- [x] Offscreen snapshot + smoke test; pinned CC0 assets (`cargo xtask assets`).

## Automation

- [x] Command registry with id/label/menu/params/enabled/run and tests.
- [x] JSON control channel: ping, list commands, dispatch, error as data.
- [x] Drives the live session (scene import, display mode, environment, framing).
- [x] `ui.screenshot` (live window capture to PNG).
- [ ] MCP server.

See the per-dimension checklists: `viewport-parity.md`, `layerstack-parity.md`, `brush-parity.md`,
`baking-parity.md`, `export-parity.md`.
