use std::sync::OnceLock;

use colored::Colorize;

use crate::cli::Depth;
use worklog_core::model::{BranchLog, Commit, ProjectLog};

static COLOR_ENABLED: OnceLock<bool> = OnceLock::new();

pub fn set_color_enabled(enabled: bool) {
    COLOR_ENABLED.set(enabled).ok();
}

pub(crate) fn color_enabled() -> bool {
    *COLOR_ENABLED.get().unwrap_or(&false)
}

pub fn render_terminal(projects: &[ProjectLog], depth: Depth) {
    if projects.is_empty() {
        eprintln!("{}", "No commits found for the given period.".dimmed());
        return;
    }

    for (i, project) in projects.iter().enumerate() {
        if i > 0 && depth != Depth::Projects {
            println!();
        }
        match depth {
            Depth::Projects => render_project_summary(project),
            Depth::Branches => render_project_with_branches(project),
            Depth::Commits => render_project(project),
        }
    }
}

fn render_project_summary(project: &ProjectLog) {
    let commits = project.total_commits();
    let branches = project.branches.len();
    let latest = project.latest_activity().unwrap_or("-");
    let summary = format!("({commits} commits, {branches} branches, {latest})").dimmed();
    if color_enabled() {
        println!("{} {}  {}", "::".bold().cyan(), project.project.bold().white(), summary);
    } else {
        println!("{} {}  {}", "::".bold(), project.project.bold(), summary);
    }
}

fn render_project_with_branches(project: &ProjectLog) {
    let latest = project.latest_activity().unwrap_or("-");
    let summary = format!("({latest})").dimmed();
    if color_enabled() {
        println!("{} {}  {}", "::".bold().cyan(), project.project.bold().white(), summary);
    } else {
        println!("{} {}  {}", "::".bold(), project.project.bold(), summary);
    }
    for branch in &project.branches {
        let count = branch.commits.len();
        let branch_latest = branch.latest_activity().unwrap_or("-");
        let branch_summary = format!("({count} commits, {branch_latest})").dimmed();
        if color_enabled() {
            println!("  {} {}  {}", ">>".green(), branch.name.green(), branch_summary);
        } else {
            println!("  {} {}  {}", ">>", branch.name, branch_summary);
        }
    }
}

pub(crate) fn render_project(project: &ProjectLog) {
    if color_enabled() {
        println!("{} {}", "::".bold().cyan(), project.project.bold().white());
    } else {
        println!("{} {}", "::".bold(), project.project.bold());
    }
    for branch in &project.branches {
        render_branch(branch);
    }
}

pub(crate) fn render_branch(branch: &BranchLog) {
    if color_enabled() {
        println!("  {} {}", ">>".green(), branch.name.green());
    } else {
        println!("  {} {}", ">>", branch.name);
    }
    render_commits(&branch.commits);
}

fn render_commits(commits: &[Commit]) {
    for commit in commits {
        let tag = commit_type_tag(commit);
        let msg = strip_type_prefix(&commit.message);
        if tag.is_empty() {
            println!(
                "    {} {} - {}  {}",
                "*".dimmed(),
                commit.hash.dimmed(),
                msg,
                commit.relative_time.dimmed(),
            );
        } else {
            println!(
                "    {} {} {} - {}  {}",
                "*".dimmed(),
                commit.hash.dimmed(),
                tag,
                msg,
                commit.relative_time.dimmed(),
            );
        }
    }
}

pub(crate) fn commit_type_tag(commit: &Commit) -> String {
    if color_enabled() {
        match commit.commit_type.as_deref() {
            Some("feat") => "feat".green().bold().to_string(),
            Some("fix") => "fix".red().bold().to_string(),
            Some("refactor") => "refactor".cyan().to_string(),
            Some("docs") => "docs".blue().to_string(),
            Some(t @ ("test" | "style")) => t.yellow().to_string(),
            Some(t @ ("chore" | "ci" | "perf" | "build")) => t.dimmed().to_string(),
            _ => String::new(),
        }
    } else {
        match commit.commit_type.as_deref() {
            Some(t) => t.to_string(),
            None => String::new(),
        }
    }
}

pub(crate) fn strip_type_prefix(message: &str) -> &str {
    if let Some(rest) = message.split_once(':') {
        rest.1.trim_start()
    } else {
        message
    }
}

pub fn render_json(projects: &[ProjectLog]) -> String {
    serde_json::to_string_pretty(projects).unwrap_or_else(|_| "[]".to_string())
}

pub fn summary_line(projects: &[ProjectLog]) -> String {
    let total_commits: usize = projects.iter().map(|p| p.total_commits()).sum();
    let total_projects = projects.len();

    match (total_commits, total_projects) {
        (0, _) => "No commits found.".to_string(),
        (1, 1) => "Found 1 commit in 1 project".to_string(),
        (c, 1) => format!("Found {c} commits in 1 project"),
        (1, p) => format!("Found 1 commit in {p} projects"),
        (c, p) => format!("Found {c} commits in {p} projects"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn make_commit(message: &str, commit_type: Option<&str>) -> Commit {
        Commit {
            hash: "abc1234".to_string(),
            message: message.to_string(),
            commit_type: commit_type.map(String::from),
            time: Local::now(),
            relative_time: "1h ago".to_string(),
        }
    }

    #[test]
    fn summary_no_commits() {
        assert_eq!(summary_line(&[]), "No commits found.");
    }

    #[test]
    fn summary_one_commit_one_project() {
        let projects = vec![ProjectLog {
            project: "test".to_string(),
            path: "/test".to_string(),
            branches: vec![BranchLog {
                name: "main".to_string(),
                commits: vec![make_commit("test", None)],
            }],
        }];
        assert_eq!(summary_line(&projects), "Found 1 commit in 1 project");
    }

    #[test]
    fn summary_multiple() {
        let projects = vec![
            ProjectLog {
                project: "a".to_string(),
                path: "/a".to_string(),
                branches: vec![BranchLog {
                    name: "main".to_string(),
                    commits: vec![make_commit("1", None), make_commit("2", None)],
                }],
            },
            ProjectLog {
                project: "b".to_string(),
                path: "/b".to_string(),
                branches: vec![BranchLog {
                    name: "main".to_string(),
                    commits: vec![make_commit("3", None)],
                }],
            },
        ];
        assert_eq!(summary_line(&projects), "Found 3 commits in 2 projects");
    }

    #[test]
    fn tag_feat_is_not_empty() {
        let commit = make_commit("feat: add feature", Some("feat"));
        assert!(!commit_type_tag(&commit).is_empty());
    }

    #[test]
    fn tag_none_is_empty() {
        let commit = make_commit("update readme", None);
        assert!(commit_type_tag(&commit).is_empty());
    }

    #[test]
    fn strip_prefix_removes_type() {
        assert_eq!(strip_type_prefix("feat: add feature"), "add feature");
        assert_eq!(strip_type_prefix("fix(auth): bug"), "bug");
    }

    #[test]
    fn strip_prefix_keeps_plain_message() {
        assert_eq!(strip_type_prefix("update readme"), "update readme");
    }
}
