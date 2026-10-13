# Baking parity checklist

Substance-3D-Painter-comparable mesh-map baking. Milestone M7.

## Mesh maps

- [ ] Tangent-space normal (from high-poly).
- [ ] Ambient occlusion.
- [ ] Curvature.
- [ ] Position.
- [ ] Thickness.
- [ ] World-space normal.
- [ ] Material ID / colour ID.

## Cage & projection

- [ ] Implicit (automatic) cage generation.
- [ ] Custom cage mesh import.
- [ ] Distance-based matching (max frontal / rear distance).
- [ ] Skew correction (M9).

## Workflow

- [ ] Baking mode UI (baker list, settings, log).
- [ ] Per-texture-set baker configuration and sync.
- [ ] Auto-rebake.
- [ ] Mesh-load state and progress.

## Correctness

- [ ] CPU and GPU bakers agree (oracle test).
- [ ] Reproducible at several resolutions.
