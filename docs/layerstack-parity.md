# Layer stack parity checklist

Substance-3D-Painter-comparable layer workflow. Landed in M3 (with M6 effects).

_Last reviewed: 2026-10-13 (M12)._

## Layers

- [x] Fill layers (constant colour; scalar value for metallic/roughness/height/AO).
- [x] Paint layers (painted from the brush tools).
- [x] Effect layers — a generator fills, a filter transforms the pixels below.
- [x] Groups with their own blending.
- [ ] Material-reference fills and projection fills.
- [ ] Layer instancing across texture sets.

## Masks

- [x] Per-layer mask (constant coverage).
- [ ] Painted masks.
- [x] Generators and filters usable as effects (smart masks still pending).
- [ ] Geometry masks.

## Blending

- [x] Standard blend modes: normal, multiply, screen, overlay, add, subtract, darken, lighten.
- [x] Opacity and visibility.
- [x] Per-channel compositing (each channel is evaluated independently).
- [ ] Per-channel blending overrides.
- [ ] Anchor points.

## Operations

- [x] Reorder, group, delete, rename.
- [ ] Duplicate.
- [ ] Flatten group / flatten copy.
- [x] Undo / redo (snapshot-based), all operations command-driven.
- [x] CPU compositor oracle; GPU compositor validated against it for flat fill stacks.
