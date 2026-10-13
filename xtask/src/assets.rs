//! Fetch pinned, permissively licensed test assets into the gitignored `corpus/` directory.
//!
//! Every asset is pinned by URL and SHA-256, so a changed or tampered upstream fails the fetch
//! rather than silently landing. Downloads shell out to `curl` (present on macOS, Linux and
//! Windows 10+). Nothing here is committed; see `ATTRIBUTION.md` for licenses.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use sha2::{Digest, Sha256};

/// A pinned asset: name, URL, expected SHA-256 (lowercase hex) and destination under `corpus/`.
struct Asset {
    name: &'static str,
    url: &'static str,
    sha256: &'static str,
    dest: &'static str,
}

/// The corpus. All assets are CC0 / public domain.
const ASSETS: &[Asset] = &[
    Asset {
        name: "Poly Haven studio_small_09 HDRI",
        url: "https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/1k/studio_small_09_1k.hdr",
        sha256: "e7cfda5f4e98e623db12b8bfd0184e048488e4855d9c83e2751fb44a32e80c45",
        dest: "hdri/studio_small_09_1k.hdr",
    },
    Asset {
        name: "Khronos glTF Sample Assets — Avocado",
        url: "https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/main/Models/Avocado/glTF-Binary/Avocado.glb",
        sha256: "ccc9c3ce56423720b09399c2351537207cd5a65f859f9e6e2f30922762f3abd4",
        dest: "models/Avocado.glb",
    },
];

/// Download every asset that is missing or fails verification.
pub fn run() -> ExitCode {
    let root = Path::new("corpus");
    if let Err(err) = std::fs::create_dir_all(root) {
        eprintln!("assets: could not create corpus/: {err}");
        return ExitCode::FAILURE;
    }

    for asset in ASSETS {
        let destination = root.join(asset.dest);
        if is_current(&destination, asset.sha256) {
            println!("assets: {} up to date ({})", asset.name, asset.dest);
            continue;
        }
        match fetch(asset, &destination) {
            Ok(()) => println!("assets: fetched {} -> {}", asset.name, destination.display()),
            Err(err) => {
                eprintln!("assets: {} — {err}", asset.name);
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

/// Whether the file exists and hashes to the expected digest.
fn is_current(path: &Path, expected: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => digest(&bytes) == expected,
        Err(_) => false,
    }
}

/// Download to a temporary file, verify the digest, then move into place.
fn fetch(asset: &Asset, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        return Err(format!("could not create {}: {err}", parent.display()));
    }

    let temporary = PathBuf::from(format!("{}.part", destination.display()));
    let status = Command::new("curl")
        .args(["--fail", "--location", "--silent", "--show-error", "--user-agent", "Substancecraft-dev (cleanroom)", "--output"])
        .arg(&temporary)
        .arg(asset.url)
        .status()
        .map_err(|err| format!("could not run curl: {err}"))?;

    if !status.success() {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("download failed with {status}"));
    }

    let bytes = std::fs::read(&temporary).map_err(|err| format!("could not read download: {err}"))?;
    let actual = digest(&bytes);
    if actual != asset.sha256 {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("sha256 mismatch: expected {}, got {actual}", asset.sha256));
    }

    std::fs::rename(&temporary, destination).map_err(|err| format!("could not finalize {}: {err}", destination.display()))?;
    Ok(())
}

/// Lowercase hex SHA-256 of a byte slice.
fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut hex = String::with_capacity(hash.len() * 2);
    for byte in hash {
        hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        hex.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap_or('0'));
    }
    hex
}
