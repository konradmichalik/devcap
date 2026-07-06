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

/// A directory is a repository root when it directly contains a `.git` directory.
fn is_repo_root(path: &Path) -> bool {
    path.join(".git").is_dir()
}

pub fn find_repos(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            // The starting directory is always walked.
            if entry.depth() == 0 {
                return true;
            }
            if !entry.file_type().is_dir() {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            if SKIP_DIRS.contains(&name.as_ref()) {
                return false;
            }
            // Once a repository root is found, stop descending: neither its
            // `.git` internals nor its working tree (which may contain vendored
            // or submodule repos) should be walked.
            match entry.path().parent() {
                Some(parent) => !is_repo_root(parent),
                None => true,
            }
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir() && is_repo_root(entry.path()))
        .map(walkdir::DirEntry::into_path)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// A self-cleaning temporary directory tree for discovery tests.
    struct TempTree {
        root: PathBuf,
    }

    impl TempTree {
        fn new() -> Self {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let mut root = std::env::temp_dir();
            root.push(format!("devcap-discovery-{}-{}", std::process::id(), id));
            fs::create_dir_all(&root).expect("create temp root");
            TempTree { root }
        }

        fn repo(&self, rel: &str) {
            fs::create_dir_all(self.root.join(rel).join(".git")).expect("create repo");
        }

        fn dir(&self, rel: &str) {
            fs::create_dir_all(self.root.join(rel)).expect("create dir");
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn finds_top_level_repos() {
        let t = TempTree::new();
        t.repo("alpha");
        t.repo("beta");
        t.dir("not-a-repo/src");
        let mut repos = find_repos(&t.root);
        repos.sort();
        assert_eq!(repos.len(), 2);
        assert!(repos.iter().any(|p| p.ends_with("alpha")));
        assert!(repos.iter().any(|p| p.ends_with("beta")));
    }

    #[test]
    fn does_not_descend_into_repo_working_tree() {
        let t = TempTree::new();
        t.repo("outer");
        // A repo nested in outer's working tree must not be reported — proving
        // discovery stops at the repo root instead of walking its whole tree.
        t.repo("outer/vendor/inner");
        let repos = find_repos(&t.root);
        assert_eq!(repos.len(), 1);
        assert!(repos[0].ends_with("outer"));
    }

    #[test]
    fn skips_ignored_directories() {
        let t = TempTree::new();
        t.repo("node_modules/pkg");
        t.repo("real");
        let repos = find_repos(&t.root);
        assert_eq!(repos.len(), 1);
        assert!(repos[0].ends_with("real"));
    }

    #[test]
    fn root_itself_is_a_repo() {
        let t = TempTree::new();
        fs::create_dir_all(t.root.join(".git")).expect("git dir");
        t.dir("src/deep");
        let repos = find_repos(&t.root);
        assert_eq!(repos, vec![t.root.clone()]);
    }
}
