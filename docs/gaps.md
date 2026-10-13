# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M5 complete)._

## Painting polish (deferred from M4–M5)

1. **No native pen pressure/tilt.** Strokes use pressure 1.0; `substancecraft-tablet` is not wired in.
2. **No physics brushes**; jitter provides scatter only.
3. **CPU painting.** Dabs stamp on the CPU; a streamed/GPU paint path is a later optimisation.
4. **Brute-force raycast.** Picking tests every triangle; needs a BVH for large meshes.
5. **Painting is not UDIM-aware** across tiles; polyfill fills a UV island (connected component).
6. **Smudge/clone sources are a pre-stroke snapshot** (no per-dab feedback).

## Layer-stack / texture-set polish

7. **GPU compositor is limited to flat fill stacks** (groups/effects/painted layers use the CPU).
8. **Effect layers are pass-through**; generators/filters arrive with M6.
9. **Masks are constants.**
10. **Undo snapshots the whole texture-set list** (including paint images); needs deltas once large.
11. **No per-set shader instances or material reassignment.**
12. **Ambient-occlusion / height display modes**; grid alignment; camera persistence; prefiltered IBL;
    selection outline.

## Next milestones

13. **No procedural content.** M6. **No baking.** M7. **No export.** M8.

## Infrastructure

14. **No perf harness** (`cargo xtask perf`, M12). **No corpus tests.**
15. **No MCP server.** The JSON control channel is the only agent interface.

## Documentation

16. Parity matrices beyond viewport/texture sets/layers/painting/automation are still 0%.
