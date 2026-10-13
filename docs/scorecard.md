# Scorecard

Measured numbers, not estimates. Generated targets (`scorecard/*.toml`, `perf/budgets.toml`) and the
`cargo xtask scorecard` task arrive with the performance milestone (M12). Until then this file is
maintained by hand.

_Last measured: 2026-10-13 (M5)._

## Workspace

| Metric | Value |
|---|---|
| Crates | 18 |
| Apps | 2 |
| Registered commands | 58 |
| Rust files / lines | 53 / ~9.2k |
| `cargo xtask ci` | green |

## Compositor / painting

| Check | Result |
|---|---|
| GPU fill-stack composite vs independent CPU reference | within 2e-3 |
| Projection paint → composite → viewport base colour | verified (`Build/m5_tools.png`) |
| Brush / eraser / polyfill tools | verified |
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
