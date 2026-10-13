# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M9 complete)._

## Workflow-depth polish (deferred from M9)

1. **One shading model.** Shader selection stores the choice, but the viewport still uses a single
   metal/rough model; the specular parameter is not consumed, and OpenPBR-specific lobes are absent.
2. **No anchor points** (reusing one layer's output in others).
3. **No layer instancing** across texture sets.
4. **No physical-size scaling** for UV projection.
5. **Post-effects are a cheap single-pass bloom + vignette** (no proper bright-pass/upsample chain, no
   colour grading).

## Export polish (deferred from M8)

6. **Per-tile UDIM content** (naming wired; content identical per tile).
7. **No USD / OpenPBR material export**, no saved preset files, no batch export across sets.
8. **No edge padding / dilation.**

## Baking polish (deferred from M7)

9. **No cage mesh / skew correction**; per-map settings; baked maps not wired into generators; CPU baker.

## Procedural / painting polish

10. **No smart materials / masks; generator/filter parameters not editable in the UI.**
11. **No native pen pressure/tilt; no physics brushes; CPU painting; not UDIM-aware.**

## Layer-stack / texture-set / viewport polish

12. **GPU compositor is limited to flat fill stacks.**
13. **Masks are constants.**
14. **Undo snapshots the whole texture-set list.**
15. **AO/height display modes; grid alignment; camera persistence; prefiltered IBL; selection outline.**

## Next milestones

16. **No native project format** (M10). **No MCP** (M11). **No perf harness / packaging** (M12).

## Infrastructure

17. **No corpus tests.**
18. **No MCP server.** The JSON control channel is the only agent interface.

## Documentation

19. Parity matrices beyond viewport/texture sets/layers/painting/procedural/baking/export/shaders/automation are still low.
