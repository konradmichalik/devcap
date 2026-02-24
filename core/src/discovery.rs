use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "target",
    ".bundle",
    "Pods",
    ".build",
    "dist",
    "build",
    ".next",
    ".cache",
];

pub fn find_repos(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            if !entry.file_type().is_dir() {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            !SKIP_DIRS.contains(&name.as_ref())
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir() && entry.file_name() == ".git")
        .filter_map(|entry| entry.path().parent().map(Path::to_path_buf))
        .collect()
}
