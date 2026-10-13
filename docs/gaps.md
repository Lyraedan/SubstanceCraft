# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M8 complete)._

## Export polish (deferred from M8)

1. **Per-tile UDIM content.** `$udim` naming is wired, but every tile writes the same image (painting is
   not UDIM-aware).
2. **No USD / OpenPBR material export**, and no `.sbsar`-style material package.
3. **No saved export presets/config files** or batch export across all texture sets.
4. **No edge padding / dilation** on export.

## Baking polish (deferred from M7)

5. **No explicit/implicit cage mesh** or skew correction.
6. **Per-map settings** are baked from shared defaults.
7. **Baked maps are not yet wired into generators/masks.**
8. **CPU baker.**

## Procedural / painting polish

9. **No smart materials / smart masks.**
10. **Filter/generator parameters are not editable in the UI** beyond defaults.
11. **No grunge library.**
12. **Effect layers are CPU-composited.**
13. **No native pen pressure/tilt; no physics brushes; CPU painting; not UDIM-aware.**

## Layer-stack / texture-set / viewport polish

14. **GPU compositor is limited to flat fill stacks.**
15. **Masks are constants.**
16. **Undo snapshots the whole texture-set list.**
17. **No per-set shader instances or material reassignment.**
18. **AO/height display modes, grid alignment, camera persistence, prefiltered IBL, selection outline.**

## Next milestones

19. **No OpenPBR/ASM shaders** (M9). **No native project format** (M10). **No MCP** (M11).

## Infrastructure

20. **No perf harness** (`cargo xtask perf`, M12). **No corpus tests.**
21. **No MCP server.** The JSON control channel is the only agent interface.

## Documentation

22. Parity matrices beyond viewport/texture sets/layers/painting/procedural/baking/export/automation are still 0%.
