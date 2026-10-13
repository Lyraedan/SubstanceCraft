# Viewport parity checklist

Substance-3D-Painter-comparable viewport behaviour. Landed in M1 and deepened in M9/M12.

_Last reviewed: 2026-10-13 (M12)._

## Camera

- [x] Orbit, pan, dolly.
- [x] Frame/focus (frame all).
- [x] Perspective projection.
- [x] Orthographic projection.
- [ ] Object/world space alignment for the grid.
- [ ] Save/restore camera per session (the camera is not part of the saved project yet).

## Shading

- [x] Metal/rough PBR (base colour, normal, metalness, roughness).
- [x] Per-set shader selection (OpenPBR / Metal-Rough / Spec-Gloss) with editable metallic, roughness,
      specular and base-colour tint.
- [ ] Distinct OpenPBR and spec/gloss shading models (one metal/rough model today; specular unused).
- [x] Image-based lighting: HDRI environment (`.hdr`/`.exr`) plus a hemisphere ambient fallback.
- [x] Tone mapping: linear, sRGB, ACES, AgX, neutral; exposure control.
- [x] Composited base colour sampled by UV from the active texture set.
- [ ] Ambient-occlusion / height channel inputs.

## Display modes

- [x] Base colour, normal, roughness, metalness.
- [x] UV checker.
- [x] Wireframe (edge overlay).
- [ ] AO, height.
- [ ] Environment only / matcap.

## Scene aids

- [x] Ground grid (tinted axes).
- [x] Background modes (solid clear, procedural sky, environment map).
- [ ] Selection outline.

## Post-effects

- [x] Bloom and vignette (single post-process pass).
- [ ] Colour grading / a full bright-pass + upsample bloom chain.

## Performance

- [x] Re-composites and re-renders only when the scene/camera/settings/size signature changes.
- [ ] GPU painting; further hot-path profiling.
