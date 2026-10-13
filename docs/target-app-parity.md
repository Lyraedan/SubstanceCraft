# Target-app parity

Where SubstanceCraft stands against Adobe Substance 3D Painter, dimension by dimension. Status
lines carry the date; update the dimension when your work moves it. SubstanceCraft is a clean-room
reimplementation: parity means comparable behaviour and results, never copied code or assets.

_Last reviewed: 2026-10-13 (M3 complete)._

| Dimension | Weight | Status | Notes |
|---|---|---|---|
| Project & texture-set workflow | high | 35% | Split by material, list/select/rename, per-set resolution + channels + UV tiles, solo (M2). |
| Layer stack & blending | high | 45% | Fill/group layers, masks, 8 blend modes, opacity, visibility, undo/redo; CPU + GPU compositor (M3). |
| Painting engine | high | 0% | Not started; M4–M5. |
| Procedural content (generators/filters/smart materials) | high | 0% | Not started; M6. |
| Baking (mesh maps + cage) | high | 0% | Not started; M7. |
| Export (presets, channels, UDIM, OpenPBR) | high | 0% | Not started; M8. |
| Viewport & PBR preview | medium | 85% | Import, camera (persp/ortho), metal/rough PBR, HDRI IBL, tone mapping, display modes, grid, wireframe, snapshot. |
| Shaders (OpenPBR / ASM) | medium | 0% | M9. |
| Automation & scripting | medium | 30% | 41 commands; control channel drives viewport, texture sets, layers and undo/redo, plus screenshots. |
| Native project format | medium | 0% | M10. |
| Performance | medium | n/a | No measurements yet. |

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
