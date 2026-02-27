# devcap-core

Core library for aggregating git commits across multiple local repositories. Powers the [devcap CLI](https://github.com/konradmichalik/devcap).

## Installation

```bash
cargo add devcap-core
```

## Usage

```rust
use std::path::Path;
use devcap_core::{discovery, git, period::Period};

// Discover all git repos under a directory
let repos = discovery::find_repos(Path::new("/Users/me/Sites"));

// Parse a time period
let range = Period::Days(7).to_time_range();

// Collect commit history per repo
for repo in &repos {
    if let Some(log) = git::collect_project_log(repo, &range, None) {
        println!("{}: {} commits", log.project, log.total_commits());
    }
}
```

## Modules

| Module | Description |
|--------|-------------|
| `discovery` | `find_repos(root)` — recursively discover git repositories, skipping build artifacts |
| `git` | `collect_project_log(repo, range, author)` — gather commits across all branches |
| `model` | `ProjectLog`, `BranchLog`, `Commit` — structured data types (Serialize) |
| `period` | `Period` enum + `TimeRange` — parse human-readable time periods (`today`, `7d`, `week`) |

## License

MIT
