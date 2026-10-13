# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M3 complete)._

## Layer-stack polish (deferred from M3)

1. **GPU compositor is limited to flat fill stacks.** Groups, effects and painted layers fall back
   to the CPU compositor ([`GpuError::Unsupported`]).
2. **The composited result does not yet drive the 3D viewport.** The layer panel shows a base-colour
   preview; wiring the composited channels into the viewport material lands with painting (M4).
3. **Effect layers are pass-through.** Generators and filters arrive with M6.
4. **Paint layers hold no content yet** (the brush engine is M4).
5. **Masks are constants.** Painted masks arrive with M4.
6. **Undo snapshots the whole texture-set list.** Fine while layer data is small; needs fine-grained
   deltas or per-set snapshots once paint images land.

## Texture-set polish (deferred from M2)

7. **No per-set shader instances or material reassignment.**
8. **UDIM handling is metadata only** (cross-tile painting needs M4).
9. **No import of pre-baked mesh maps** or per-set descriptions.

## Viewport polish (deferred from M1)

10. **Ambient-occlusion / height display modes.**
11. **Grid object/world alignment and camera persistence.**
12. **Specular IBL is a mip approximation** (no prefiltered GGX chain / BRDF LUT).
13. **Wireframe is an edge overlay**, not polygon-mode wireframe.
14. **No selection outline.**

## Next milestones

15. **No painting.** M4–M5.
16. **No procedural content.** M6.
17. **No baking.** M7.
18. **No export.** M8.

## Infrastructure

19. **No perf harness.** `cargo xtask perf` and budgets arrive with M12.
20. **No corpus tests.** `cargo xtask assets` fetches pinned CC0 assets; no oracle tests consume them
    yet.

## Automation

21. **No MCP server.** The JSON control channel is the only agent interface so far.

## Documentation

22. Parity matrices beyond viewport/texture sets/layers/automation are still 0%.
