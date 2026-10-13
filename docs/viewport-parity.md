# Viewport parity checklist

Substance-3D-Painter-comparable viewport behaviour. Milestone M1.

_Last reviewed: 2026-10-13 (M1 complete)._

## Camera

- [x] Orbit, pan, dolly.
- [x] Frame/focus (frame all).
- [x] Perspective projection.
- [x] Orthographic projection.
- [ ] Object/world space alignment for the grid.
- [ ] Save/restore camera per session (persisted with M10).

## Shading

- [x] Metal/rough PBR (base colour, normal, metalness, roughness).
- [ ] Ambient-occlusion channel input.
- [x] Image-based lighting: HDRI environment (`.hdr`/`.exr`) plus a hemisphere ambient fallback.
- [x] Tone mapping: linear, sRGB, ACES, AgX, neutral; exposure control.
- [ ] Spec/gloss workflow (M9).
- [ ] OpenPBR shader (M9).

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
