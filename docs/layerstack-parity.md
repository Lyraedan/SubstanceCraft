# Layer stack parity checklist

Substance-3D-Painter-comparable layer workflow. Landed in M3 (with M6 effects).

_Last reviewed: 2026-10-13 (M12)._

## Layers

- [x] Fill layers with sources: uniform colour/scalar, imported image, procedural generator and baked
      mesh map.
- [x] Fill projection: UV, planar and tri-planar, with tiling, rotation, offset, axis and per-channel
      strength.
- [x] Paint layers (painted from the brush tools).
- [x] Effect layers — a generator fills, a filter transforms the pixels below.
- [x] Groups with their own blending.
- [x] Per-layer connected-part restriction (restrict a layer's coverage to a set of mesh pieces).
- [ ] Material-reference fills.
- [ ] Layer instancing across texture sets.

## Masks

- [x] Per-layer mask: constant coverage or a per-texel effect stack (fill, generator, filter, baked
      mesh map, projected image) evaluated with invert.
- [ ] Painted (brush) masks.
- [x] Generators and filters usable as mask effects (saved smart masks still pending).
- [ ] Geometry / part masks.

## Blending

- [x] 26 blend modes: normal, multiply, screen, overlay, add, subtract, darken, lighten, color
      dodge/burn, linear burn, hard/soft/vivid/linear/pin light, hard mix, difference, exclusion,
      divide, and the non-separable hue/saturation/color/luminosity/darker-color/lighter-color.
- [x] Opacity and visibility.
- [x] Per-channel compositing (each channel is evaluated independently).
- [x] Per-channel opacity overrides (a per-channel multiplier on the layer opacity).
- [ ] Anchor points.

## Operations

- [x] Reorder, group, delete, rename.
- [ ] Duplicate.
- [ ] Flatten group / flatten copy.
- [x] Undo / redo (snapshot-based), all operations command-driven.
- [x] CPU compositor oracle; GPU compositor validated against it for flat fill stacks.
