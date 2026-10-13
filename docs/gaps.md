# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M4 complete)._

## Painting polish (deferred from M4)

1. **No native pen pressure/tilt.** Strokes use pressure 1.0; `substancecraft-tablet` is not wired
   into the viewport yet.
2. **Brush radius is in texture pixels**, not screen space (so coverage depends on UV density).
3. **Painting targets the top paint layer.** There is no active-layer selection in the session yet,
   and no eraser tool; erase is done with a normal layer or mask.
4. **CPU painting.** Dabs are stamped on the CPU; a GPU/streamed paint path is a later optimisation.
5. **No per-layer paint blend for scalar channels** beyond the fill model; paint layers only carry
   colour.
6. **Brute-force raycast.** Picking tests every triangle; needs a BVH for large meshes.
7. **No UDIM-aware painting** across tiles.

## Layer-stack polish (deferred from M3)

8. **GPU compositor is limited to flat fill stacks** (groups/effects/painted layers use the CPU).
9. **Effect layers are pass-through**; generators/filters arrive with M6.
10. **Masks are constants.**
11. **Undo snapshots the whole texture-set list** (including paint images); needs deltas once images
    are large.

## Texture-set / viewport polish

12. **No per-set shader instances or material reassignment.**
13. **Ambient-occlusion / height display modes**; grid object/world alignment; camera persistence.
14. **Specular IBL is a mip approximation**; wireframe is an edge overlay; no selection outline.

## Next milestones

15. **No painting tools** beyond the brush (projection/polyfill/smudge/clone/particle). M5.
16. **No procedural content.** M6. **No baking.** M7. **No export.** M8.

## Infrastructure

17. **No perf harness** (`cargo xtask perf`, M12). **No corpus tests.**
18. **No MCP server.** The JSON control channel is the only agent interface.

## Documentation

19. Parity matrices beyond viewport/texture sets/layers/painting/automation are still 0%.
