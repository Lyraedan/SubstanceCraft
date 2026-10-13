# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M7 complete)._

## Baking polish (deferred from M7)

1. **No explicit/implicit cage mesh** export or skew correction; projection uses a fixed normal offset.
2. **No edge padding / dilation** of the baked result across UV seams.
3. **Per-map settings** (AO distance, thickness distance, cage size) are baked from shared defaults.
4. **Baked maps are not yet wired into generators/masks** (no curvature/AO-driven grunge input).
5. **CPU baker**; a GPU baker is a later optimisation.

## Procedural-content polish (deferred from M6)

6. **No smart materials / smart masks.**
7. **Filter/generator parameters are not editable in the UI** beyond defaults.
8. **No grunge library or directional content.**
9. **Effect layers are CPU-composited.**

## Painting polish (deferred from M4–M5)

10. **No native pen pressure/tilt.**
11. **No physics brushes.**
12. **CPU painting**; streamed/GPU paint is later.
13. **Painting is not UDIM-aware.**

## Layer-stack / texture-set / viewport polish

14. **GPU compositor is limited to flat fill stacks.**
15. **Masks are constants.**
16. **Undo snapshots the whole texture-set list.**
17. **No per-set shader instances or material reassignment.**
18. **AO/height display modes, grid alignment, camera persistence, prefiltered IBL, selection outline.**

## Next milestones

19. **No export.** M8. **No OpenPBR/ASM shaders** (M9). **No native project format** (M10).

## Infrastructure

20. **No perf harness** (`cargo xtask perf`, M12). **No corpus tests.**
21. **No MCP server.** The JSON control channel is the only agent interface.

## Documentation

22. Parity matrices beyond viewport/texture sets/layers/painting/procedural/baking/automation are still 0%.
