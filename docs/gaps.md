# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M1 complete)._

## Viewport polish (deferred from M1)

1. **Ambient-occlusion / height display modes.** Only base colour, normal, roughness, metalness,
   UV checker and wireframe are wired.
2. **Grid object/world alignment and camera persistence.** The grid is world-space; the camera is
   not yet saved per session (arrives with the project format, M10).
3. **Specular IBL is a mip approximation.** The environment uses box-filtered mips rather than a
   prefiltered GGX chain and a BRDF LUT; good enough for preview, worth improving for export fidelity.
4. **Wireframe is an edge overlay**, not a polygon-mode wireframe (Metal has no line polygon mode).
5. **Selection outline** is not implemented.

## Next milestones

6. **No texture sets.** The mesh is one scene with materials; M2 splits it into texture sets with
   per-set channels and shaders, and handles UDIM.
7. **No layer stack.** M3.
8. **No painting.** M4–M5.
9. **No procedural content.** M6.
10. **No baking.** M7.
11. **No export.** M8.

## Infrastructure

12. **No perf harness.** `cargo xtask perf` and budgets arrive with M12. The viewport re-renders every
    frame (polled at ~5 Hz when idle); it should render on revision change only.
13. **No corpus tests.** `cargo xtask assets` fetches pinned CC0 assets; no oracle/round-trip corpus
    tests consume them yet (they come with baking/export).

## Automation

14. **No MCP server.** The JSON control channel is the only agent interface so far.

## Documentation

15. Parity matrices beyond the viewport and automation areas are still 0% and will fill in as their
    milestones land.
