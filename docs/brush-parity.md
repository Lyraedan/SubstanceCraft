# Brush and painting parity checklist

Substance-3D-Painter-comparable painting. Landed in M4–M5.

_Last reviewed: 2026-10-13 (M12)._

## Brush engine

- [x] Dabs with spacing, size, flow and opacity.
- [x] Pressure scaling (size and opacity) — mouse reports pressure 1.0.
- [ ] Tilt and native pen pressure through `substancecraft-tablet`.
- [x] Hardness/falloff curves.
- [x] Jitter (particle-like scatter).
- [x] Symmetry (horizontal, vertical, both).
- [x] Screen-space sizing (texels-per-screen-pixel from the hit triangle's UV density).
- [x] Alpha tips (an image shapes the dab).
- [x] Lazy mouse (the dab lags the pointer by a configurable radius).

## Tools

- [x] Brush, eraser.
- [x] Smudge.
- [x] Clone (with offset).
- [x] Polygon fill (UV-island `polyfill`).
- [ ] Projection tool (paint an image through a placed plane).
- [ ] Particle and physics brushes.

## Painting model

- [x] Projection painting through BVH raycasts onto the mesh's UVs.
- [x] Paints into the active layer's texture target; one undo step per stroke.
- [x] Active-layer target (`layer.select`) with a paint-layer fallback.
- [x] Live composited base colour in the viewport.
- [ ] UDIM-aware painting across tiles.
- [ ] GPU/streamed painting.

## Presets and colour

- [x] Save/load brush presets (`brush.save_preset` / `brush.load_preset`; in-session for now).
- [x] Eyedropper (samples the composited base colour; viewport `P` or `paint.eyedropper`).
- [ ] Swatches / palettes.
- [x] Brush colour and symmetry are commands and persist with the project.
- [x] Straight-line strokes (viewport Shift-click or `paint.stroke_line`).
