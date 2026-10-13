# Scorecard

Measured numbers, not estimates. Generated targets (`scorecard/*.toml`, `perf/budgets.toml`) and the
`cargo xtask scorecard` task arrive with the performance milestone (M12). Until then this file is
maintained by hand.

_Last measured: 2026-10-13 (M7)._

## Workspace

| Metric | Value |
|---|---|
| Crates | 18 |
| Apps | 2 |
| Registered commands | 67 |
| Rust files / lines | 56 / ~10.9k |
| `cargo xtask ci` | green |

## Compositor / painting / content / baking

| Check | Result |
|---|---|
| GPU fill-stack composite vs independent CPU reference | within 2e-3 |
| BVH ray query vs brute force | identical distance |
| Generators deterministic and in range | Perlin/Simplex/Voronoi/White |
| Effect layers (generator fill, filter transform, opacity) | verified |
| Projection paint → composite → viewport base colour | verified |
| Brush / eraser / polyfill tools | verified |
| Bake 7 maps at 128², 8 AO samples | 0.93 s (release: faster) |
| A stroke is a single undo step | verified |

## Performance budgets

None enforced yet. `cargo xtask perf` and `perf/budgets.toml` arrive with M12. Budgets to establish:

- Viewport frame time on a 2M-triangle mesh at 1080p.
- Layer-stack compositing time per texture set at 2K and 4K.
- Bake time for AO/normal at 2K on a representative high-poly.
- Cold start to interactive viewport.

## Corpus floors

`cargo xtask assets` fetches pinned, SHA-256-verified CC0 assets into `corpus/` (a Poly Haven HDRI and
the Khronos Avocado glTF). No regression floors are asserted yet.

## Dead settings

None (few settings exist yet).
