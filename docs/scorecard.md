# Scorecard

Measured numbers, not estimates. Generated targets (`scorecard/*.toml`, `perf/budgets.toml`) and the
`cargo xtask scorecard` task arrive with the performance milestone (M12). Until then this file is
maintained by hand.

_Last measured: 2026-10-13 (M11)._

## Workspace

| Metric | Value |
|---|---|
| Crates | 18 |
| Apps | 2 |
| Registered commands | 85 |
| Rust files / lines | 62 / ~12.7k |
| `cargo xtask ci` | green |

## Compositor / painting / content / baking / export / shaders / projects / automation

| Check | Result |
|---|---|
| GPU fill-stack composite vs independent CPU reference | within 2e-3 |
| BVH ray query vs brute force | identical distance |
| Generators deterministic and in range | Perlin/Simplex/Voronoi/White |
| Projection paint → composite → viewport base colour | verified |
| Bake 7 maps at 256², 16 AO samples (release) | 0.43 s |
| Export channel round-trip (PNG) | within 1/255 |
| `.sccraft` round-trip (bytes + file), version rejection | verified |
| MCP `tools/list` / `tools/call` | 85 tools; `app_version`, `scene_info` verified |
| CLI `script` runner | executes requests in one session |

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
