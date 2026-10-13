# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M1 core)._

## Finishing the viewport (M1 loose ends)

1. **No HDRI environment.** Lighting is a hemisphere ambient plus a key light. Need EXR/HDR loading
   (through `substancecraft-codecs`) and a prefiltered environment for image-based lighting.
2. **No ground grid/floor.** Scene aids help orientation while painting.
3. **No wireframe or orthographic view.** Useful for UV and topology checks.
4. **`cargo xtask assets` is a stub.** Need pinned, sha256-verified CC0 meshes/HDRIs into `corpus/`.
5. **No live `ui.screenshot`.** The offscreen `snapshot` example exists; the control channel cannot
   yet capture the running UI.

## Next milestones

6. **No texture sets.** The mesh is one scene with materials; M2 splits it into texture sets with
   per-set channels and shaders, and handles UDIM.
7. **No layer stack.** M3.
8. **No painting.** M4–M5.
9. **No procedural content.** M6.
10. **No baking.** M7.
11. **No export.** M8.

## Infrastructure

12. **No perf harness.** `cargo xtask perf` and budgets arrive with M12. The viewport renders every
    frame; it should render on revision change.
13. **No corpus.** Real-file oracles arrive with the baking/export milestones.

## Automation

14. **No MCP server.** The JSON control channel is the only agent interface so far.

## Documentation

15. Parity matrices beyond the viewport and automation areas are still 0% and will fill in as their
    milestones land.
