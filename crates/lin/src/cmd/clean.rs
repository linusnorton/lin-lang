use std::path::PathBuf;

#[derive(clap::Args)]
pub struct CleanArgs {
    /// Root directory to search (default: ".")
    pub path: Option<PathBuf>,
}

pub fn run(args: &CleanArgs) {
    let root = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));
    clean_dir(&root);
}

fn clean_dir(dir: &std::path::Path) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some(".lin-cache") {
                match std::fs::remove_dir_all(&path) {
                    Ok(()) => eprintln!("Removed {}", path.display()),
                    Err(e) => eprintln!("Failed to remove {}: {}", path.display(), e),
                }
            } else {
                clean_dir(&path);
            }
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".test-bin") || name.contains("__run_tmp") {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}
