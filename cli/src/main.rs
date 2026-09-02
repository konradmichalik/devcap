mod cli;
mod clipboard;
mod config;
mod interactive;
mod output;

use std::cmp::Reverse;
use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate};
use clap::Parser;
use devcap_core::{
    discovery, git, model,
    period::{Period, TimeRange},
};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let cfg = config::load();

    let range = resolve_time_range(cli.since, cli.until, cli.period, &cfg)?;

    let path = cli.path.or(cfg.path).unwrap_or_else(|| PathBuf::from("."));
    let author = cli.author.or(cfg.author).or_else(git::default_author);
    let show_origin = cli.show_origin || cfg.show_origin.unwrap_or(false);
    let with_stat = cli.stat || cfg.stat.unwrap_or(false);

    let use_color = if cli.no_color || cli.json {
        false
    } else if let Some(cfg_color) = cfg.color {
        cfg_color
    } else {
        std::io::stdout().is_terminal()
    };
    output::set_color_enabled(use_color);
    let author_ref = author.as_deref();

    let spinner = if !cli.json {
        let sp = ProgressBar::new_spinner();
        if let Ok(style) = ProgressStyle::default_spinner()
            .tick_strings(&[
                "\u{2802}", "\u{2816}", "\u{2834}", "\u{2830}", "\u{2860}", "\u{28e0}", "\u{28c0}",
                "\u{2880}",
            ])
            .template("{spinner} {msg}")
        {
            sp.set_style(style);
        }
        sp.set_message("Scanning repositories...");
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let repos = discovery::find_repos(&path);

    if repos.is_empty() {
        if let Some(sp) = &spinner {
            sp.finish_and_clear();
        }
        if cli.json {
            println!("[]");
        } else {
            eprintln!("No git repositories found in: {}", path.display());
        }
        return Ok(());
    }

    let mut projects: Vec<_> = repos
        .par_iter()
        .filter_map(|repo| git::collect_project_log(repo, &range, author_ref, with_stat))
        .collect();

    let sort_spec = cli
        .sort
        .or_else(|| {
            cfg.sort
                .as_deref()
                .and_then(|s| s.parse::<cli::SortSpec>().ok())
        })
        .unwrap_or_default();

    sort_projects(&mut projects, sort_spec);

    if let Some(sp) = &spinner {
        sp.finish_with_message(format!("\u{2713} {}", output::summary_line(&projects)));
    }

    if cli.interactive {
        interactive::run(&projects, show_origin)?;
    } else if cli.json {
        println!("{}", output::render_json(&projects));
    } else {
        if !projects.is_empty() {
            println!();
        }
        output::render_terminal(&projects, cli.depth, show_origin);
    }

    if cli.copy {
        let text = clipboard::render_plain(&projects, cli.depth, show_origin);
        match arboard::Clipboard::new() {
            Ok(mut cb) => {
                if let Err(e) = cb.set_text(&text) {
                    eprintln!("Warning: could not copy to clipboard: {e}");
                } else {
                    eprintln!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Warning: clipboard unavailable: {e}"),
        }
    }

    Ok(())
}

fn parse_config_date(value: Option<&str>, field: &str) -> Option<NaiveDate> {
    let s = value?;
    match s.parse::<NaiveDate>() {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("Warning: invalid {field} in ~/.devcap.toml: \"{s}\" ({e})");
            None
        }
    }
}

fn resolve_time_range(
    cli_since: Option<NaiveDate>,
    cli_until: Option<NaiveDate>,
    cli_period: Option<Period>,
    cfg: &config::DevcapConfig,
) -> Result<TimeRange> {
    let since = cli_since.or_else(|| parse_config_date(cfg.since.as_deref(), "since"));
    let until = cli_until.or_else(|| parse_config_date(cfg.until.as_deref(), "until"));

    let resolve_period = || {
        cli_period
            .or_else(|| cfg.period.as_deref().and_then(|s| s.parse::<Period>().ok()))
            .unwrap_or(Period::Today)
    };

    match (since, until) {
        (Some(s), Some(u)) => TimeRange::from_dates(s, u).map_err(|e| anyhow::anyhow!(e)),
        (Some(s), None) => TimeRange::from_since_date(s).map_err(|e| anyhow::anyhow!(e)),
        (None, Some(u)) => {
            let range = resolve_period().to_time_range();
            range.with_until_date(u).map_err(|e| anyhow::anyhow!(e))
        }
        (None, None) => Ok(resolve_period().to_time_range()),
    }
}

fn latest_commit_time(p: &model::ProjectLog) -> Option<DateTime<Local>> {
    p.branches
        .iter()
        .flat_map(|br| br.commits.first())
        .map(|c| c.time)
        .max()
}

fn commit_count(p: &model::ProjectLog) -> usize {
    p.branches.iter().map(|br| br.commits.len()).sum()
}

fn line_count(p: &model::ProjectLog) -> u64 {
    p.branches
        .iter()
        .flat_map(|br| &br.commits)
        .filter_map(|c| c.diff_stat.as_ref())
        .map(|s| (s.insertions + s.deletions) as u64)
        .sum()
}

fn project_name_key(p: &model::ProjectLog) -> String {
    p.project.to_lowercase()
}

/// Sort in place by a cached key so each project's key is computed once (O(n))
/// instead of on every comparison. `Desc` wraps the key in `Reverse`, which
/// keeps the stable sort's original ordering among equal keys.
fn sort_by_key_dir<T, K, F>(items: &mut [T], dir: cli::SortDirection, key: F)
where
    K: Ord,
    F: Fn(&T) -> K,
{
    match dir {
        cli::SortDirection::Asc => items.sort_by_cached_key(key),
        cli::SortDirection::Desc => items.sort_by_cached_key(|x| Reverse(key(x))),
    }
}

fn sort_projects(projects: &mut [model::ProjectLog], spec: cli::SortSpec) {
    match spec.field {
        cli::SortField::Time => sort_by_key_dir(projects, spec.direction, latest_commit_time),
        cli::SortField::Commits => sort_by_key_dir(projects, spec.direction, commit_count),
        cli::SortField::Name => sort_by_key_dir(projects, spec.direction, project_name_key),
        cli::SortField::Lines => sort_by_key_dir(projects, spec.direction, line_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_by_key_dir_asc_orders_ascending() {
        let mut v = vec![3, 1, 2, 1];
        sort_by_key_dir(&mut v, cli::SortDirection::Asc, |x| *x);
        assert_eq!(v, vec![1, 1, 2, 3]);
    }

    #[test]
    fn sort_by_key_dir_desc_is_stable_for_ties() {
        // (key, tag) — the tag records original order among equal keys.
        let mut v = vec![(1, 'a'), (2, 'b'), (1, 'c'), (2, 'd')];
        sort_by_key_dir(&mut v, cli::SortDirection::Desc, |x| x.0);
        // Keys descending; equal keys keep their original relative order.
        assert_eq!(v, vec![(2, 'b'), (2, 'd'), (1, 'a'), (1, 'c')]);
    }

    fn project(name: &str, n: usize) -> model::ProjectLog {
        let commits = (0..n)
            .map(|i| model::Commit {
                hash: format!("h{i}"),
                message: "m".to_string(),
                commit_type: None,
                time: Local::now(),
                committer_time: Local::now(),
                relative_time: "now".to_string(),
                url: None,
                diff_stat: None,
            })
            .collect();
        model::ProjectLog {
            project: name.to_string(),
            path: String::new(),
            origin: None,
            remote_url: None,
            branches: vec![model::BranchLog {
                name: "main".to_string(),
                url: None,
                commits,
                diff_stat: None,
            }],
            diff_stat: None,
        }
    }

    #[test]
    fn sort_projects_by_name_is_case_insensitive_ascending() {
        let mut projects = vec![project("Zeta", 1), project("alpha", 1), project("Mango", 1)];
        sort_projects(&mut projects, "name".parse().expect("valid spec"));
        let names: Vec<&str> = projects.iter().map(|p| p.project.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Mango", "Zeta"]);
    }

    #[test]
    fn sort_projects_by_commits_descending() {
        let mut projects = vec![project("a", 1), project("b", 5), project("c", 3)];
        sort_projects(&mut projects, "commits".parse().expect("valid spec"));
        let names: Vec<&str> = projects.iter().map(|p| p.project.as_str()).collect();
        assert_eq!(names, vec!["b", "c", "a"]);
    }
}
