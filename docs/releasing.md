# Releasing

Signing and per-platform packaging follow the shared Craft release playbook
(`../craftrules/release/playbook.md`). This document records SubstanceCraft's specifics and grows
with the release milestone (M12).

## Versioning

The version lives in `[workspace.package] version` in the root `Cargo.toml`. `README.md`,
`ROADMAP.md` and `NOTICE` reference the product, not a version, so a bump touches one file.

## Building

```sh
cargo xtask build     # release build, apps copied into Build/
```

## Packaging (target)

- **macOS**: `Build/SubstanceCraft.app` bundle, codesigned and notarised; bundle id
  `ai.storyteller.substancecraft`.
- **Windows**: portable `.exe` plus an installer.
- **Linux**: tarball (and later a Flatpak/AppImage).

Package scripts land in `packaging/` with M12.
