# Baking parity checklist

Substance-3D-Painter-comparable mesh-map baking. Landed in M7.

_Last reviewed: 2026-10-13 (M12)._

## Mesh maps

- [x] Tangent-space normal (from a high-poly mesh).
- [x] Ambient occlusion (cosine-hemisphere sampling).
- [x] Curvature (normal-field divergence).
- [x] Position.
- [x] Thickness.
- [x] World-space normal.
- [x] Material ID / colour ID.

## Cage & projection

- [ ] Explicit / implicit cage mesh export and editing.
- [ ] Distance-based matching (max frontal / rear distance) as a separate setting.
- [ ] Skew correction.

## Workflow

- [x] Bake panel: resolution, AO samples, high-poly import, run, per-map preview and PNG save.
- [x] A bake log with timing and notes.
- [ ] Per-texture-set baker configuration and sync.
- [ ] Auto-rebake and progress.

## Correctness

- [x] BVH ray queries validated against brute force.
- [x] Reproducible (seeded AO); bakes at 128² and 256² verified.
- [ ] Baked maps wired into generators/masks.
- [ ] A GPU baker (CPU today).
