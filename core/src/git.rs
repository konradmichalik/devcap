use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

use crate::model::{BranchLog, Commit, ProjectLog, RepoOrigin};
use crate::period::TimeRange;

pub fn default_author() -> Option<String> {
    Command::new("git")
        .args(["config", "--global", "user.name"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
}

fn list_branches(repo: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo.to_string_lossy(),
            "branch",
            "--format=%(refname:short)",
        ])
        .output()
        .context("Failed to run git branch")?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

fn log_branch(
    repo: &Path,
    branch: &str,
    range: &TimeRange,
    author: Option<&str>,
) -> Result<Vec<Commit>> {
    let since_str = range.since.to_rfc3339();

    let mut args = vec![
        "-C".to_string(),
        repo.to_string_lossy().to_string(),
        "log".to_string(),
        branch.to_string(),
        format!("--after={since_str}"),
        "--format=%h%x00%s%x00%aI".to_string(),
        "--no-merges".to_string(),
    ];

    if let Some(until) = &range.until {
        args.push(format!("--before={}", until.to_rfc3339()));
    }

    if let Some(author) = author {
        args.push(format!("--author={author}"));
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to run git log")?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let now = Local::now();

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| parse_commit_line(line, now))
        .collect())
}

fn parse_commit_line(line: &str, now: DateTime<Local>) -> Option<Commit> {
    let parts: Vec<&str> = line.splitn(3, '\0').collect();
    if parts.len() != 3 {
        return None;
    }

    let time = DateTime::parse_from_rfc3339(parts[2])
        .ok()?
        .with_timezone(&Local);

    Some(Commit {
        hash: parts[0].to_string(),
        message: parts[1].to_string(),
        commit_type: detect_commit_type(parts[1]),
        relative_time: format_relative(now, time),
        time,
    })
}

fn detect_commit_type(message: &str) -> Option<String> {
    let prefix = message.split([':', '(']).next()?;
    let trimmed = prefix.trim();
    match trimmed {
        "feat" | "fix" | "refactor" | "docs" | "test" | "chore" | "perf" | "ci" | "build"
        | "style" => Some(trimmed.to_string()),
        _ => None,
    }
}

fn format_relative(now: DateTime<Local>, then: DateTime<Local>) -> String {
    let duration = now.signed_duration_since(then);
    let mins = duration.num_minutes();

    if mins < 1 {
        "just now".to_string()
    } else if mins < 60 {
        format!("{mins}m ago")
    } else if mins < 1440 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}

fn get_remote_url(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if url.is_empty() { None } else { Some(url) }
}

fn extract_hostname(url: &str) -> Option<&str> {
    // SSH: git@github.com:user/repo.git
    if let Some(rest) = url.strip_prefix("git@") {
        return rest.split(':').next();
    }
    // SSH variant: ssh://git@host/...
    if let Some(rest) = url.strip_prefix("ssh://") {
        let after_at = rest.split('@').last()?;
        return after_at.split('/').next().map(|h| h.split(':').next().unwrap_or(h));
    }
    // HTTPS: https://github.com/user/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        let without_scheme = url.split("://").nth(1)?;
        let after_auth = without_scheme.split('@').last()?;
        return after_auth.split('/').next();
    }
    None
}

fn classify_host(hostname: &str) -> RepoOrigin {
    let lower = hostname.to_lowercase();
    if lower == "github.com" {
        RepoOrigin::GitHub
    } else if lower == "gitlab.com" {
        RepoOrigin::GitLab
    } else if lower == "bitbucket.org" {
        RepoOrigin::Bitbucket
    } else if lower.contains("gitlab") {
        RepoOrigin::GitLabSelfHosted
    } else {
        RepoOrigin::Custom(hostname.to_string())
    }
}

/// Convert a git remote URL (SSH or HTTPS) into a browser-friendly HTTPS URL.
pub fn remote_to_browser_url(raw: &str) -> Option<String> {
    let mut url = raw.trim().to_string();

    // SSH: git@github.com:user/repo.git → https://github.com/user/repo
    if url.starts_with("git@") {
        url = url.replacen("git@", "https://", 1);
        if let Some(pos) = url.find(':') {
            // Only replace the first colon after the host (not in https://)
            let after_scheme = &url["https://".len()..];
            if let Some(colon) = after_scheme.find(':') {
                let abs = "https://".len() + colon;
                url.replace_range(abs..abs + 1, "/");
            } else {
                url.replace_range(pos..pos + 1, "/");
            }
        }
    }

    // ssh://git@host/... → https://host/...
    if url.starts_with("ssh://") {
        url = url.replacen("ssh://", "https://", 1);
        if let Some(at) = url.find('@') {
            url = format!("https://{}", &url[at + 1..]);
        }
    }

    if url.ends_with(".git") {
        url.truncate(url.len() - 4);
    }

    if url.starts_with("https://") || url.starts_with("http://") {
        Some(url)
    } else {
        None
    }
}

pub fn browser_url(repo: &Path) -> Option<String> {
    let raw = get_remote_url(repo)?;
    remote_to_browser_url(&raw)
}

pub fn detect_origin(repo: &Path) -> Option<RepoOrigin> {
    let url = get_remote_url(repo)?;
    let hostname = extract_hostname(&url)?;
    Some(classify_host(hostname))
}

pub fn collect_project_log(
    repo: &Path,
    range: &TimeRange,
    author: Option<&str>,
) -> Option<ProjectLog> {
    let project_name = repo.file_name()?.to_string_lossy().to_string();
    let branches = list_branches(repo).ok()?;

    let mut branch_logs: Vec<BranchLog> = branches
        .into_iter()
        .filter_map(|branch_name| {
            let commits = log_branch(repo, &branch_name, range, author).ok()?;
            if commits.is_empty() {
                None
            } else {
                Some(BranchLog {
                    name: branch_name,
                    commits,
                })
            }
        })
        .collect();

    if branch_logs.is_empty() {
        return None;
    }

    branch_logs.sort_by(|a, b| {
        let a_primary = is_primary_branch(&a.name);
        let b_primary = is_primary_branch(&b.name);
        b_primary.cmp(&a_primary).then_with(|| a.name.cmp(&b.name))
    });

    Some(ProjectLog {
        project: project_name,
        path: repo.to_string_lossy().to_string(),
        origin: detect_origin(repo),
        remote_url: browser_url(repo),
        branches: branch_logs,
    })
}

fn is_primary_branch(name: &str) -> bool {
    matches!(name, "main" | "master")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn format_relative_just_now() {
        let now = Local::now();
        assert_eq!(format_relative(now, now), "just now");
    }

    #[test]
    fn format_relative_minutes() {
        let now = Local::now();
        let then = now - Duration::minutes(5);
        assert_eq!(format_relative(now, then), "5m ago");
    }

    #[test]
    fn format_relative_hours() {
        let now = Local::now();
        let then = now - Duration::hours(3);
        assert_eq!(format_relative(now, then), "3h ago");
    }

    #[test]
    fn format_relative_days() {
        let now = Local::now();
        let then = now - Duration::days(2);
        assert_eq!(format_relative(now, then), "2d ago");
    }

    #[test]
    fn detect_feat() {
        assert_eq!(
            detect_commit_type("feat: add spinner"),
            Some("feat".to_string())
        );
    }

    #[test]
    fn detect_fix() {
        assert_eq!(
            detect_commit_type("fix: off-by-one error"),
            Some("fix".to_string())
        );
    }

    #[test]
    fn detect_scoped() {
        assert_eq!(
            detect_commit_type("feat(auth): add OAuth"),
            Some("feat".to_string())
        );
    }

    #[test]
    fn detect_none_for_regular_message() {
        assert_eq!(detect_commit_type("update README"), None);
    }

    #[test]
    fn detect_none_for_empty() {
        assert_eq!(detect_commit_type(""), None);
    }

    #[test]
    fn parse_commit_line_valid() {
        let now = Local::now();
        let time_str = now.to_rfc3339();
        let line = format!("abc1234\x00feat: add feature\x00{time_str}");
        let commit = parse_commit_line(&line, now);
        assert!(commit.is_some());
        let c = commit.unwrap_or_else(|| panic!("Expected Some"));
        assert_eq!(c.hash, "abc1234");
        assert_eq!(c.message, "feat: add feature");
        assert_eq!(c.commit_type, Some("feat".to_string()));
    }

    #[test]
    fn parse_commit_line_invalid() {
        let now = Local::now();
        assert!(parse_commit_line("incomplete line", now).is_none());
    }

    #[test]
    fn primary_branch_detected() {
        assert!(is_primary_branch("main"));
        assert!(is_primary_branch("master"));
        assert!(!is_primary_branch("feature/auth"));
        assert!(!is_primary_branch("develop"));
    }

    #[test]
    fn extract_hostname_https() {
        assert_eq!(
            extract_hostname("https://github.com/user/repo.git"),
            Some("github.com")
        );
        assert_eq!(
            extract_hostname("https://gitlab.com/group/project"),
            Some("gitlab.com")
        );
    }

    #[test]
    fn extract_hostname_http() {
        assert_eq!(
            extract_hostname("http://gitea.local/org/repo"),
            Some("gitea.local")
        );
    }

    #[test]
    fn extract_hostname_ssh_git_at() {
        assert_eq!(
            extract_hostname("git@github.com:user/repo.git"),
            Some("github.com")
        );
        assert_eq!(
            extract_hostname("git@gitlab.company.de:group/project.git"),
            Some("gitlab.company.de")
        );
    }

    #[test]
    fn extract_hostname_ssh_scheme() {
        assert_eq!(
            extract_hostname("ssh://git@bitbucket.org/team/repo.git"),
            Some("bitbucket.org")
        );
        assert_eq!(
            extract_hostname("ssh://git@gitlab.internal:2222/group/repo.git"),
            Some("gitlab.internal")
        );
    }

    #[test]
    fn extract_hostname_https_with_auth() {
        assert_eq!(
            extract_hostname("https://token@github.com/user/repo.git"),
            Some("github.com")
        );
    }

    #[test]
    fn extract_hostname_empty() {
        assert_eq!(extract_hostname(""), None);
        assert_eq!(extract_hostname("not-a-url"), None);
    }

    #[test]
    fn classify_github() {
        assert_eq!(classify_host("github.com"), RepoOrigin::GitHub);
        assert_eq!(classify_host("GitHub.com"), RepoOrigin::GitHub);
    }

    #[test]
    fn classify_gitlab() {
        assert_eq!(classify_host("gitlab.com"), RepoOrigin::GitLab);
    }

    #[test]
    fn classify_bitbucket() {
        assert_eq!(classify_host("bitbucket.org"), RepoOrigin::Bitbucket);
    }

    #[test]
    fn classify_gitlab_self_hosted() {
        assert_eq!(
            classify_host("gitlab.company.de"),
            RepoOrigin::GitLabSelfHosted
        );
        assert_eq!(
            classify_host("gitlab.internal"),
            RepoOrigin::GitLabSelfHosted
        );
    }

    #[test]
    fn classify_custom() {
        assert_eq!(
            classify_host("gitea.local"),
            RepoOrigin::Custom("gitea.local".to_string())
        );
        assert_eq!(
            classify_host("codeberg.org"),
            RepoOrigin::Custom("codeberg.org".to_string())
        );
    }
}
