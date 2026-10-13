# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M10 complete)._

## Project-format polish (deferred from M10)

1. **RON text is large.** Paint images serialise as text f32 arrays; a binary container (or embedded
   PNG/KTX) is needed for real projects.
2. **No project templates or New Project window** — only a blank new project.
3. **No startup recovery prompt.** Autosave exists but recovery is a manual command.
4. **The environment map, high-poly mesh and baked maps are not saved** with the project.

## Workflow-depth polish (deferred from M9)

5. **One shading model.** Shader selection is stored but the viewport uses one metal/rough model; the
   specular parameter is unused.
6. **No anchor points, layer instancing, or physical-size scaling.**
7. **Post-effects are a cheap single-pass bloom + vignette.**

## Export polish (deferred from M8)

8. **Per-tile UDIM content** (naming wired; content identical per tile).
9. **No USD / OpenPBR material export**, saved presets, batch export, or edge padding.

## Baking polish (deferred from M7)

10. **No cage mesh / skew correction**; per-map settings; baked maps not wired into generators; CPU baker.

## Procedural / painting / layer / viewport polish

11. **No smart materials / masks; generator/filter parameters not editable in the UI.**
12. **No native pen pressure/tilt; no physics brushes; CPU painting; not UDIM-aware.**
13. **GPU compositor limited to flat fill stacks; masks are constants; undo snapshots the whole set list.**
14. **AO/height display modes; grid alignment; camera persistence; prefiltered IBL; selection outline.**

## Next milestones

15. **No MCP** (M11). **No perf harness / packaging** (M12).

## Infrastructure

16. **No corpus tests.**
17. **No MCP server.** The JSON control channel is the only agent interface.

## Documentation

18. Parity matrices beyond viewport/texture sets/layers/painting/procedural/baking/export/shaders/project/automation are still partial.
