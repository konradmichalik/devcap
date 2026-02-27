use anyhow::Result;
use colored::Colorize;
use dialoguer::console::{self, strip_ansi_codes};
use dialoguer::theme::{ColorfulTheme, Theme};
use dialoguer::FuzzySelect;
use std::fmt;
use std::process::Command;

use crate::output;
use devcap_core::model::{BranchLog, Commit, ProjectLog};

const BACK_LABEL: &str = "\u{276e} Back";
const QUIT_LABEL: &str = "\u{276e} Quit";
const SHOW_ALL_LABEL: &str = "\u{2630} Show all";

struct DevcapTheme {
    inner: ColorfulTheme,
}

impl DevcapTheme {
    fn new() -> Self {
        Self {
            inner: ColorfulTheme::default(),
        }
    }
}

impl Theme for DevcapTheme {
    fn format_fuzzy_select_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        search_term: &str,
        cursor_pos: usize,
    ) -> fmt::Result {
        self.inner
            .format_fuzzy_select_prompt(f, prompt, search_term, cursor_pos)
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        if active {
            let clean = strip_ansi_codes(text);
            write!(
                f,
                "  {} {}",
                console::style("\u{276f}").for_stderr().bold(),
                console::style(clean.as_ref()).for_stderr().bold()
            )
        } else {
            write!(f, "    {text}")
        }
    }
}

enum Selection {
    Back,
    ShowAll,
    Index(usize),
}

pub fn run(projects: &[ProjectLog], show_origin: bool) -> Result<()> {
    let theme = DevcapTheme::new();

    loop {
        match select_project(&theme, projects, show_origin)? {
            Selection::Back => return Ok(()),
            Selection::ShowAll => {
                println!();
                output::render_terminal(projects, crate::cli::Depth::Commits, show_origin);
                println!();
            }
            Selection::Index(idx) => {
                let project = &projects[idx];
                browse_project(&theme, project, show_origin)?;
            }
        }
    }
}

fn browse_project(theme: &DevcapTheme, project: &ProjectLog, show_origin: bool) -> Result<()> {
    loop {
        match select_branch(theme, project)? {
            Selection::Back => return Ok(()),
            Selection::ShowAll => {
                println!();
                output::render_project(project, show_origin);
                println!();
            }
            Selection::Index(idx) => {
                let branch = &project.branches[idx];
                browse_branch(theme, project, branch)?;
            }
        }
    }
}

fn browse_branch(theme: &DevcapTheme, project: &ProjectLog, branch: &BranchLog) -> Result<()> {
    loop {
        match select_commit(theme, branch)? {
            Selection::Back => return Ok(()),
            Selection::ShowAll => {
                println!();
                output::render_branch(branch);
                println!();
            }
            Selection::Index(idx) => {
                let commit = &branch.commits[idx];
                show_commit_detail(project, commit)?;
            }
        }
    }
}

fn select_project(
    theme: &DevcapTheme,
    projects: &[ProjectLog],
    show_origin: bool,
) -> Result<Selection> {
    let items: Vec<String> = [QUIT_LABEL, SHOW_ALL_LABEL]
        .into_iter()
        .map(String::from)
        .chain(projects.iter().map(|p| format_project_item(p, show_origin)))
        .collect();

    parse_selection(
        FuzzySelect::with_theme(theme)
            .with_prompt("Select project")
            .items(&items)
            .default(0)
            .interact_opt()?,
    )
}

fn select_branch(theme: &DevcapTheme, project: &ProjectLog) -> Result<Selection> {
    let items: Vec<String> = [BACK_LABEL, SHOW_ALL_LABEL]
        .into_iter()
        .map(String::from)
        .chain(project.branches.iter().map(format_branch_item))
        .collect();

    parse_selection(
        FuzzySelect::with_theme(theme)
            .with_prompt("Select branch")
            .items(&items)
            .default(0)
            .interact_opt()?,
    )
}

fn select_commit(theme: &DevcapTheme, branch: &BranchLog) -> Result<Selection> {
    let items: Vec<String> = [BACK_LABEL, SHOW_ALL_LABEL]
        .into_iter()
        .map(String::from)
        .chain(branch.commits.iter().map(format_commit_item))
        .collect();

    parse_selection(
        FuzzySelect::with_theme(theme)
            .with_prompt("Select commit")
            .items(&items)
            .default(0)
            .interact_opt()?,
    )
}

fn parse_selection(result: Option<usize>) -> Result<Selection> {
    Ok(match result {
        Some(0) | None => Selection::Back,
        Some(1) => Selection::ShowAll,
        Some(i) => Selection::Index(i - 2),
    })
}

fn show_commit_detail(project: &ProjectLog, commit: &Commit) -> Result<()> {
    let output = Command::new("git")
        .args([
            "-C",
            &project.path,
            "show",
            "--stat",
            "--format=medium",
            &commit.hash,
        ])
        .output()?;

    if output.status.success() {
        println!("\n{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!("Failed to show commit {}", commit.hash);
    }

    Ok(())
}

fn format_project_item(project: &ProjectLog, show_origin: bool) -> String {
    let commits = project.total_commits();
    let branches = project.branches.len();
    let latest = project.latest_activity().unwrap_or("-");
    let origin = output::origin_tag(project, show_origin);
    let summary = format!(
        "({} {}, {} {}, {})",
        commits,
        pluralize("commit", commits),
        branches,
        pluralize("branch", branches),
        latest,
    )
    .dimmed();
    if output::color_enabled() {
        format!(
            "{} {}{}  {}",
            "::".bold().cyan(),
            project.project.bold().white(),
            origin,
            summary
        )
    } else {
        format!(
            "{} {}{}  {}",
            "::".bold(),
            project.project.bold(),
            origin,
            summary
        )
    }
}

fn format_branch_item(branch: &BranchLog) -> String {
    let commits = branch.commits.len();
    let latest = branch.latest_activity().unwrap_or("-");
    let summary = format!("({} {}, {})", commits, pluralize("commit", commits), latest,).dimmed();
    if output::color_enabled() {
        format!("{} {}  {}", ">>".green(), branch.name.green(), summary)
    } else {
        format!("{} {}  {}", ">>", branch.name, summary)
    }
}

fn format_commit_item(commit: &Commit) -> String {
    let tag = output::commit_type_tag(commit);
    let msg = output::strip_type_prefix(&commit.message);
    if tag.is_empty() {
        format!(
            "{} - {}  {}",
            commit.hash.dimmed(),
            msg,
            commit.relative_time.dimmed(),
        )
    } else {
        format!(
            "{} {} - {}  {}",
            commit.hash.dimmed(),
            tag,
            msg,
            commit.relative_time.dimmed(),
        )
    }
}

fn pluralize(word: &str, count: usize) -> String {
    if count == 1 {
        return word.to_string();
    }
    match word {
        "branch" => "branches".to_string(),
        other => format!("{other}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BranchLog, Commit, ProjectLog};
    use chrono::Local;

    fn make_commit(hash: &str, message: &str, relative: &str) -> Commit {
        Commit {
            hash: hash.to_string(),
            message: message.to_string(),
            commit_type: None,
            time: Local::now(),
            relative_time: relative.to_string(),
        }
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut in_escape = false;
        for c in s.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if c == 'm' {
                    in_escape = false;
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn format_project_single() {
        let project = ProjectLog {
            project: "my-app".to_string(),
            path: "/test".to_string(),
            origin: None,
            remote_url: None,
            branches: vec![BranchLog {
                name: "main".to_string(),
                commits: vec![make_commit("abc", "msg", "1h ago")],
            }],
        };
        let text = strip_ansi(&format_project_item(&project, false));
        assert!(text.contains("my-app"));
        assert!(text.contains("1 commit"));
        assert!(text.contains("1 branch"));
    }

    #[test]
    fn format_project_plural() {
        let project = ProjectLog {
            project: "my-app".to_string(),
            path: "/test".to_string(),
            origin: None,
            remote_url: None,
            branches: vec![
                BranchLog {
                    name: "main".to_string(),
                    commits: vec![
                        make_commit("a", "m1", "1h ago"),
                        make_commit("b", "m2", "2h ago"),
                    ],
                },
                BranchLog {
                    name: "dev".to_string(),
                    commits: vec![make_commit("c", "m3", "3h ago")],
                },
            ],
        };
        let text = strip_ansi(&format_project_item(&project, false));
        assert!(text.contains("3 commits"));
        assert!(text.contains("2 branches"));
    }

    #[test]
    fn format_project_with_origin() {
        use devcap_core::model::RepoOrigin;
        let project = ProjectLog {
            project: "my-app".to_string(),
            path: "/test".to_string(),
            origin: Some(RepoOrigin::GitHub),
            remote_url: None,
            branches: vec![BranchLog {
                name: "main".to_string(),
                commits: vec![make_commit("abc", "msg", "1h ago")],
            }],
        };
        let text = strip_ansi(&format_project_item(&project, true));
        assert!(text.contains("[GitHub]"));
    }

    #[test]
    fn format_project_origin_hidden_when_flag_off() {
        use devcap_core::model::RepoOrigin;
        let project = ProjectLog {
            project: "my-app".to_string(),
            path: "/test".to_string(),
            origin: Some(RepoOrigin::GitHub),
            remote_url: None,
            branches: vec![BranchLog {
                name: "main".to_string(),
                commits: vec![make_commit("abc", "msg", "1h ago")],
            }],
        };
        let text = strip_ansi(&format_project_item(&project, false));
        assert!(!text.contains("[GitHub]"));
    }

    #[test]
    fn format_branch_singular() {
        let branch = BranchLog {
            name: "feature/auth".to_string(),
            commits: vec![make_commit("a", "m", "1h ago")],
        };
        let text = strip_ansi(&format_branch_item(&branch));
        assert!(text.contains("feature/auth"));
        assert!(text.contains("1 commit"));
    }

    #[test]
    fn format_branch_plural() {
        let branch = BranchLog {
            name: "main".to_string(),
            commits: vec![
                make_commit("a", "m1", "1h ago"),
                make_commit("b", "m2", "2h ago"),
            ],
        };
        let text = strip_ansi(&format_branch_item(&branch));
        assert!(text.contains("main"));
        assert!(text.contains("2 commits"));
    }

    #[test]
    fn format_commit_display() {
        let commit = make_commit("abc1234", "feat: add auth", "2h ago");
        let text = strip_ansi(&format_commit_item(&commit));
        assert!(text.contains("abc1234"));
        assert!(text.contains("add auth"));
        assert!(text.contains("2h ago"));
    }

    #[test]
    fn pluralize_singular() {
        assert_eq!(pluralize("commit", 1), "commit");
        assert_eq!(pluralize("branch", 1), "branch");
    }

    #[test]
    fn pluralize_plural() {
        assert_eq!(pluralize("commit", 0), "commits");
        assert_eq!(pluralize("commit", 5), "commits");
        assert_eq!(pluralize("branch", 2), "branches");
    }
}
