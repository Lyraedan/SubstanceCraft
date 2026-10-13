# Viewport parity checklist

Substance-3D-Painter-comparable viewport behaviour. Milestone M1 (core landed).

_Last reviewed: 2026-10-13 (M1 core)._

## Camera

- [x] Orbit, pan, dolly.
- [x] Frame/focus (frame all).
- [x] Perspective projection.
- [ ] Orthographic projection.
- [ ] Object/world space alignment for the grid.
- [ ] Save/restore camera per session.

## Shading

- [x] Metal/rough PBR (base colour, normal, metalness, roughness).
- [ ] Ambient-occlusion channel input.
- [x] Image-based-style lighting (hemisphere ambient + key light).
- [ ] HDRI environment map (EXR/HDR) with prefiltered IBL.
- [x] Tone mapping: linear, sRGB, ACES, AgX, neutral; exposure control.
- [ ] Spec/gloss workflow (M9).
- [ ] OpenPBR shader (M9).

## Display modes

- [x] Base colour, normal, roughness, metalness.
- [x] UV checker.
- [ ] AO, height.
- [ ] Wireframe; wireframe-on-shaded.
- [ ] Environment only / matcap.

## Scene aids

- [ ] Ground grid and floor.
- [ ] Background modes (environment, solid colour, gradient).
- [ ] Selection outline.
