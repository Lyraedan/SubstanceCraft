# Attribution

Every third-party and original asset is listed here with its path, title, author, source and license.
Assets must be permissively licensed and keep their license file next to them.

## Libraries

| Component | Path | Author | Source | License |
|---|---|---|---|---|
| craft-libs (craft-ui, craft-settings, craft-atomic, craft-quit) | git pinned rev `666c358065261a2816b376fa3bb10538dbf53040` | ArtCraft Team and craft-libs contributors | https://github.com/storytold/craft-libs | MIT OR Apache-2.0 |

## Test assets

Pinned, permissively licensed models and HDRIs are fetched into `corpus/` (gitignored) by
`cargo xtask assets`, which verifies each file's SHA-256. None are committed.

| Asset | Path (in `corpus/`) | Author | Source | License | SHA-256 |
|---|---|---|---|---|---|
| studio_small_09 HDRI (1k) | `hdri/studio_small_09_1k.hdr` | Poly Haven | https://polyhaven.com/a/studio_small_09 | CC0 1.0 | `e7cfda5f4e98e623db12b8bfd0184e048488e4855d9c83e2751fb44a32e80c45` |
| Avocado (glTF) | `models/Avocado.glb` | Microsoft (Khronos glTF Sample Assets) | https://github.com/KhronosGroup/glTF-Sample-Assets | CC0 1.0 | `ccc9c3ce56423720b09399c2351537207cd5a65f859f9e6e2f30922762f3abd4` |

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
