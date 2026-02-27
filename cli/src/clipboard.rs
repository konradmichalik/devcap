use crate::cli::Depth;
use crate::output;
use devcap_core::model::ProjectLog;

/// Render projects as clean plain text without ANSI codes.
pub fn render_plain(projects: &[ProjectLog], depth: Depth, show_origin: bool) -> String {
    if projects.is_empty() {
        return "No commits found for the given period.".to_string();
    }
    let mut out = String::new();
    for (i, project) in projects.iter().enumerate() {
        if i > 0 && depth != Depth::Projects {
            out.push('\n');
        }
        match depth {
            Depth::Projects => render_project_summary(&mut out, project, show_origin),
            Depth::Branches => render_project_branches(&mut out, project, show_origin),
            Depth::Commits => render_project_full(&mut out, project, show_origin),
        }
    }
    out
}

fn origin_suffix(project: &ProjectLog, show_origin: bool) -> String {
    if !show_origin {
        return String::new();
    }
    match &project.origin {
        Some(origin) => format!(" [{origin}]"),
        None => String::new(),
    }
}

fn render_project_summary(out: &mut String, project: &ProjectLog, show_origin: bool) {
    let commits = project.total_commits();
    let branches = project.branches.len();
    let latest = project.latest_activity().unwrap_or("-");
    let origin = origin_suffix(project, show_origin);
    out.push_str(&format!(
        ":: {}{}  ({commits} commits, {branches} branches, {latest})\n",
        project.project, origin
    ));
}

fn render_project_branches(out: &mut String, project: &ProjectLog, show_origin: bool) {
    let latest = project.latest_activity().unwrap_or("-");
    let origin = origin_suffix(project, show_origin);
    out.push_str(&format!(":: {}{}  ({latest})\n", project.project, origin));
    for branch in &project.branches {
        let count = branch.commits.len();
        let branch_latest = branch.latest_activity().unwrap_or("-");
        out.push_str(&format!(
            "  >> {}  ({count} commits, {branch_latest})\n",
            branch.name
        ));
    }
}

fn render_project_full(out: &mut String, project: &ProjectLog, show_origin: bool) {
    let origin = origin_suffix(project, show_origin);
    out.push_str(&format!(":: {}{}\n", project.project, origin));
    for branch in &project.branches {
        out.push_str(&format!("  >> {}\n", branch.name));
        for commit in &branch.commits {
            let tag = match commit.commit_type.as_deref() {
                Some(t) => format!("{t} - "),
                None => String::new(),
            };
            let msg = output::strip_type_prefix(&commit.message);
            out.push_str(&format!(
                "    * {} {}{msg}  {}\n",
                commit.hash, tag, commit.relative_time
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use devcap_core::model::{BranchLog, Commit, ProjectLog, RepoOrigin};

    fn make_commit(hash: &str, msg: &str, commit_type: Option<&str>) -> Commit {
        Commit {
            hash: hash.to_string(),
            message: msg.to_string(),
            commit_type: commit_type.map(String::from),
            time: Local::now(),
            relative_time: "1h ago".to_string(),
        }
    }

    fn make_project(name: &str, origin: Option<RepoOrigin>) -> ProjectLog {
        ProjectLog {
            project: name.to_string(),
            path: format!("/test/{name}"),
            origin,
            branches: vec![BranchLog {
                name: "main".to_string(),
                commits: vec![
                    make_commit("abc1234", "feat: add login", Some("feat")),
                    make_commit("def5678", "fix: resolve crash", Some("fix")),
                ],
            }],
        }
    }

    #[test]
    fn plain_output_contains_no_ansi() {
        let projects = vec![make_project("my-app", None)];
        let text = render_plain(&projects, Depth::Commits, false);
        assert!(!text.contains('\x1b'));
        assert!(text.contains("my-app"));
        assert!(text.contains("main"));
        assert!(text.contains("abc1234"));
    }

    #[test]
    fn empty_projects_returns_message() {
        let text = render_plain(&[], Depth::Commits, false);
        assert_eq!(text, "No commits found for the given period.");
    }

    #[test]
    fn projects_depth_shows_summary_only() {
        let projects = vec![make_project("repo", None)];
        let text = render_plain(&projects, Depth::Projects, false);
        assert!(text.contains(":: repo"));
        assert!(text.contains("2 commits"));
        assert!(!text.contains(">> main"));
    }

    #[test]
    fn branches_depth_shows_branches() {
        let projects = vec![make_project("repo", None)];
        let text = render_plain(&projects, Depth::Branches, false);
        assert!(text.contains(":: repo"));
        assert!(text.contains(">> main"));
        assert!(!text.contains("abc1234"));
    }

    #[test]
    fn commits_depth_shows_full_tree() {
        let projects = vec![make_project("repo", None)];
        let text = render_plain(&projects, Depth::Commits, false);
        assert!(text.contains(":: repo"));
        assert!(text.contains(">> main"));
        assert!(text.contains("abc1234"));
        assert!(text.contains("feat - add login"));
        assert!(text.contains("fix - resolve crash"));
    }

    #[test]
    fn origin_shown_when_enabled() {
        let projects = vec![make_project("repo", Some(RepoOrigin::GitHub))];
        let text = render_plain(&projects, Depth::Projects, true);
        assert!(text.contains("[GitHub]"));
    }

    #[test]
    fn origin_hidden_when_disabled() {
        let projects = vec![make_project("repo", Some(RepoOrigin::GitHub))];
        let text = render_plain(&projects, Depth::Projects, false);
        assert!(!text.contains("[GitHub]"));
    }

    #[test]
    fn commit_without_type_has_no_tag() {
        let projects = vec![ProjectLog {
            project: "test".to_string(),
            path: "/test".to_string(),
            origin: None,
            branches: vec![BranchLog {
                name: "main".to_string(),
                commits: vec![make_commit("aaa1111", "update readme", None)],
            }],
        }];
        let text = render_plain(&projects, Depth::Commits, false);
        assert!(text.contains("aaa1111 update readme"));
    }
}
