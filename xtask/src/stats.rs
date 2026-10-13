//! Workspace statistics: crate counts, source size and registered commands.

use std::process::ExitCode;

/// Print workspace statistics.
pub fn run() -> ExitCode {
    let crates = count_dirs("crates");
    let apps = count_dirs("apps");

    let mut files = 0usize;
    let mut lines = 0usize;
    for root in ["crates", "apps", "xtask/src"] {
        collect_rust(root, &mut files, &mut lines);
    }

    let commands = count_occurrences("crates/engine/src", "CommandSpec::new(");

    println!("SubstanceCraft stats");
    println!("  crates:   {crates}");
    println!("  apps:     {apps}");
    println!("  rs files: {files}");
    println!("  rs lines: {lines}");
    println!("  commands: {commands}");
    ExitCode::SUCCESS
}

fn count_dirs(root: &str) -> usize {
    std::fs::read_dir(root).map(|entries| entries.filter_map(Result::ok).filter(|entry| entry.path().is_dir()).count()).unwrap_or(0)
}

fn collect_rust(path: &str, files: &mut usize, lines: &mut usize) {
    let Ok(entries) = std::fs::read_dir(path) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust(&path.to_string_lossy(), files, lines);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            *files += 1;
            if let Ok(contents) = std::fs::read_to_string(&path) {
                *lines += contents.lines().count();
            }
        }
    }
}

fn count_occurrences(root: &str, needle: &str) -> usize {
    let mut count = 0usize;
    let Ok(entries) = std::fs::read_dir(root) else { return 0 };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            count += count_occurrences(&path.to_string_lossy(), needle);
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && let Ok(contents) = std::fs::read_to_string(&path)
        {
            count += contents.matches(needle).count();
        }
    }
    count
}
