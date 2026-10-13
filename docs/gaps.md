# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M2 complete)._

## Texture-set polish (deferred from M2)

1. **No per-set shader instances.** Every texture set shares the one shader; assigning a distinct
   shader per set is M9.
2. **No material reassignment.** A set's material list is fixed by the import; reassigning sets to
   materials is not implemented.
3. **UDIM handling is metadata only.** Tiles are detected and editable, but nothing paints across
   tiles yet (needs M4).
4. **No import of pre-baked mesh maps** or per-set descriptions.

## Viewport polish (deferred from M1)

5. **Ambient-occlusion / height display modes.**
6. **Grid object/world alignment and camera persistence** (persistence with M10).
7. **Specular IBL is a mip approximation** (no prefiltered GGX chain / BRDF LUT).
8. **Wireframe is an edge overlay**, not polygon-mode wireframe.
9. **No selection outline.**

## Next milestones

10. **No layer stack.** M3.
11. **No painting.** M4–M5.
12. **No procedural content.** M6.
13. **No baking.** M7.
14. **No export.** M8.

## Infrastructure

15. **No perf harness.** `cargo xtask perf` and budgets arrive with M12. The viewport re-renders every
    frame (polled at ~5 Hz when idle); it should render on revision change only.
16. **No corpus tests.** `cargo xtask assets` fetches pinned CC0 assets; no oracle/round-trip corpus
    tests consume them yet (they come with baking/export).

## Automation

17. **No MCP server.** The JSON control channel is the only agent interface so far.

## Documentation

18. Parity matrices beyond viewport/texture sets/automation are still 0% and will fill in as their
    milestones land.
