# Export parity checklist

Substance-3D-Painter-comparable texture export. Milestone M8.

## Channels & formats

- [ ] Per-channel export (base colour, roughness, metalness, normal, height, AO, …).
- [ ] 8/16-bit PNG and TGA; 16/32-bit EXR.
- [ ] Channel packing into combined maps.
- [ ] sRGB / linear assignment per channel.

## Presets & naming

- [ ] Built-in export presets (Unreal, Unity, glTF, OpenPBR, spec/gloss).
- [ ] Custom presets.
- [ ] Naming conventions (`$textureSet_$channel`).
- [ ] Output templates and subfolders.

## Scope

- [ ] UDIM / UV-tile export.
- [ ] Resolution up to 8K.
- [ ] Export flattened layers/masks/groups directly.
- [ ] OpenPBR material export via USD (M9).

## Correctness

- [ ] Exported maps re-import to identical values (round-trip test).
- [ ] CPU oracle and GPU compositor agree on flattened output.
