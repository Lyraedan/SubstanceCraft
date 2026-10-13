# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M6 complete)._

## Procedural-content polish (deferred from M6)

1. **No smart materials / smart masks** (saved layer-stack snippets) yet.
2. **Filter/generator parameters are not editable in the UI** beyond defaults (seed/scale/radius are
   settable through commands only).
3. **No grunge library, curvature/AO-driven generators, or directional/scratches content.**
4. **Effect layers are CPU-composited**; the GPU compositor reports them unsupported.

## Painting polish (deferred from M4–M5)

5. **No native pen pressure/tilt.** Strokes use pressure 1.0.
6. **No physics brushes**; jitter provides scatter only.
7. **CPU painting.** Dabs stamp on the CPU; a streamed/GPU paint path is a later optimisation.
8. **Brute-force raycast.** Picking tests every triangle; needs a BVH for large meshes.
9. **Painting is not UDIM-aware**; polyfill fills a UV island.
10. **Smudge/clone sources are a pre-stroke snapshot.**

## Layer-stack / texture-set polish

11. **GPU compositor is limited to flat fill stacks.**
12. **Masks are constants.**
13. **Undo snapshots the whole texture-set list** (including paint images).
14. **No per-set shader instances or material reassignment.**
15. **AO/height display modes**; grid alignment; camera persistence; prefiltered IBL; selection outline.

## Next milestones

16. **No baking.** M7. **No export.** M8.

## Infrastructure

17. **No perf harness** (`cargo xtask perf`, M12). **No corpus tests.**
18. **No MCP server.** The JSON control channel is the only agent interface.

## Documentation

19. Parity matrices beyond viewport/texture sets/layers/painting/procedural/automation are still 0%.
