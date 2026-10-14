# Releasing

Signing and per-platform packaging follow the shared Craft release playbook
(`../craftrules/release/playbook.md`; if unavailable, keep this file as the local record). **Packaging
is not implemented yet** — M12 delivered the performance harness and scorecard, and left release
packaging deferred (see `docs/gaps.md`). This document records what exists and the target shape.

The release pipeline itself is already wired: `.github/workflows/release.yml` calls the shared
[storytold/craft-common-actions](https://github.com/storytold/craft-common-actions) reusable
`release.yml` (`app-name: substancecraft`, `app-title: SubstanceCraft`, `craft-fonts-ref` pinned). It
is **dormant** until the `packaging/` scripts land: the shared pipeline reads the version from
`packaging/env.sh` and runs `packaging/<platform>/package.sh`, so those are the remaining pieces.
FreeBSD nightly testing uses the shared `freebsd.yml`; CI uses the shared
`checkout-craft-fonts` composite action.

## Versioning

The version lives in `[workspace.package] version` in the root `Cargo.toml`. `README.md`, `ROADMAP.md`
and `NOTICE` reference the product, not a version, so a bump touches one file.

## Building

```sh
cargo xtask build     # cargo build --release, apps copied into Build/
cargo xtask ci        # the full gate before building
cargo xtask perf      # confirm performance budgets
```

`Build/` holds the finished binaries (`substancecraft`, `substancecraft-cli`); it is gitignored.

## Packaging (target, not implemented)

- **macOS**: a `Build/SubstanceCraft.app` bundle, codesigned and notarised; bundle id
  `ai.storyteller.substancecraft`.
- **Windows**: a portable `.exe` plus an installer.
- **Linux**: a tarball (and later a Flatpak/AppImage).

When this lands, package scripts go in `packaging/` and are driven by `cargo xtask build`.
