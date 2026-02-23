# worklog-git

Aggregate git commits across multiple local repositories for daily stand-ups and time tracking.

Scans a directory tree for git repos in parallel, filters commits by author and time period, and renders a colorized `Project -> Branch -> Commits` tree — or structured JSON.

> [!NOTE]
> Requires `git` on `$PATH`. Author defaults to `git config --global user.name`.

## 🔥 Installation

### Homebrew (macOS)

```bash
brew install konradmichalik/tap/worklog-git
```

### From source

```bash
cargo install --path .
```

## 🚀 Quick Start

```bash
# Today's commits in the current directory
worklog-git

# Yesterday across all projects under ~/Sites
worklog-git -p yesterday --path ~/Sites

# Last 7 days, filtered by author
worklog-git -p 7d --path ~/Sites -a "Jane Doe"

# This calendar week as JSON
worklog-git -p week --json
```

## 📋 Example Output

```
:: my-app
  >> main
    * a1b2c3d feat - add login flow  3h ago
    * b2c3d4e fix - resolve token refresh race condition  5h ago
    * c3d4e5f refactor - extract auth middleware  6h ago
  >> feature/notifications
    * d4e5f6a feat - add push notification service  2h ago

:: design-system
  >> main
    * e5f6a7b docs - update component API reference  1h ago
    * f6a7b8c test - add snapshot tests for Button  4h ago
    * 7b8c9d0 update color palette for dark mode  7h ago

Found 7 commits in 2 projects
```

Conventional commit types are color-highlighted: `feat` green, `fix` red, `refactor` cyan, `docs` blue, `test`/`style` yellow, `chore`/`ci`/`perf`/`build` dimmed.

## ✨ Features

- **Flexible time periods** — `today`, `yesterday`, `week`, or arbitrary `Xh` / `Xd` (e.g. `24h`, `3d`, `14d`)
- **Parallel repo scanning** — uses [rayon](https://github.com/rayon-rs/rayon); skips `node_modules`, `target`, `vendor`, and other build artifacts automatically
- **Conventional commit highlighting** — color-coded by type in terminal output
- **JSON output** — machine-readable, suitable for scripting or further processing

## ⚙️ Options

```
Usage: worklog-git [OPTIONS]

Options:
  -p, --period <PERIOD>  Time period: today, yesterday, 24h, 3d, 7d, week [default: today]
      --path <PATH>      Root directory to scan for git repos [default: .]
      --json             Output as JSON instead of colored terminal tree
  -a, --author <AUTHOR>  Filter by author name (defaults to git config user.name)
  -h, --help             Print help
  -V, --version          Print version
```

> [!TIP]
> Use `--json` to pipe into `jq` for custom filtering:
> ```bash
> worklog-git -p week --json | jq '[.[] | {project, commits: [.branches[].commits[].message]}]'
> ```

## 💡 JSON Schema

Each entry in the JSON array follows this shape:

```json
{
  "project": "my-app",
  "path": "/Users/me/Sites/my-app",
  "branches": [
    {
      "name": "main",
      "commits": [
        {
          "hash": "a1b2c3d",
          "message": "feat: add login flow",
          "commit_type": "feat",
          "timestamp": "2026-02-23T10:15:00+01:00",
          "relative_time": "3h ago"
        }
      ]
    }
  ]
}
```

> [!IMPORTANT]
> Merge commits are excluded from all output (`--no-merges` is always applied).

## 📜 License

MIT
