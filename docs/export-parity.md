# Export parity checklist

Substance-3D-Painter-comparable texture export. Landed in M8.

_Last reviewed: 2026-10-13 (M12)._

## Channels & formats

- [x] Per-channel export (base colour, metallic, roughness, normal, height, AO, emissive — where enabled).
- [x] PNG 8-bit, PNG 16-bit, TGA 8-bit and EXR 32-bit float.
- [x] Channel packing into combined maps (any channel into R/G/B/A).
- [x] Linear data written without gamma (data maps round-trip exactly).
- [ ] Explicit sRGB/linear assignment per channel.

## Presets & naming

- [x] Built-in presets: OpenPBR, glTF, Unreal (ORM), Unity (mask), Spec/Gloss.
- [ ] Custom presets saved to disk.
- [x] Naming templates (`$textureSet`, `$channel`, `$udim`).
- [ ] Output templates and subfolders.

## Scope

- [x] UDIM naming (`$udim`, one file per tile).
- [ ] Per-tile UDIM content (painting is not UDIM-aware yet).
- [x] Resolution up to 8K (clamped 16..8192).
- [ ] Flattened layers/masks/groups exported directly.
- [ ] OpenPBR material export via USD.

## Correctness

- [x] Exported maps re-import to identical values (round-trip test, within 1/255).
- [x] ORM packing test.
- [ ] CPU oracle and GPU compositor agree on flattened output.
