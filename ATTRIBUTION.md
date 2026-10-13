# Attribution

Every third-party and original asset is listed here with its path, title, author, source and license.
Assets must be permissively licensed and keep their license file next to them.

## Libraries

| Component | Path | Author | Source | License |
|---|---|---|---|---|
| craft-libs (craft-ui, craft-settings, craft-atomic, craft-quit) | git pinned rev `666c358065261a2816b376fa3bb10538dbf53040` | ArtCraft Team and craft-libs contributors | https://github.com/storytold/craft-libs | MIT OR Apache-2.0 |

## Test assets

Pinned, permissively licensed models, HDRIs and textures are fetched into `corpus/` (gitignored) by
`cargo xtask assets` — implemented with the viewport milestone. None are committed.

## Icons and brand

Original SubstanceCraft icons live in `docs/brand/`. No Adobe icons are used.

## Provenance / clean-room notes

SubstanceCraft is a clean-room reimplementation. Behaviour and appearance were informed by observing
Substance 3D Painter; implementation is from public specifications only:

- OpenPBR Surface 1.1 (ASWF)
- glTF 2.0 specification (Khronos)
- MikkTSpace tangent-space generation
- UDIM / UV-tile conventions
- Standard metal/rough and spec/gloss PBR literature
