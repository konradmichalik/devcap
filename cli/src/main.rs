mod cli;
mod interactive;
mod output;

use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use worklog_core::{discovery, git, model};

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    output::set_color_enabled(cli.color);
    let range = cli.period.to_time_range();
    let author = cli.author.or_else(git::default_author);
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

    let repos = discovery::find_repos(&cli.path);

    if repos.is_empty() {
        if let Some(sp) = &spinner {
            sp.finish_and_clear();
        }
        if cli.json {
            println!("[]");
        } else {
            eprintln!("No git repositories found in: {}", cli.path.display());
        }
        return Ok(());
    }

    let mut projects: Vec<_> = repos
        .par_iter()
        .filter_map(|repo| git::collect_project_log(repo, &range, author_ref))
        .collect();

    projects.sort_by(|a, b| {
        let latest = |p: &model::ProjectLog| {
            p.branches
                .iter()
                .flat_map(|br| br.commits.first())
                .map(|c| c.time)
                .max()
        };
        latest(b).cmp(&latest(a))
    });

    if let Some(sp) = &spinner {
        sp.finish_with_message(format!("\u{2713} {}", output::summary_line(&projects)));
    }

    if cli.interactive {
        interactive::run(&projects)?;
    } else if cli.json {
        println!("{}", output::render_json(&projects));
    } else {
        if !projects.is_empty() {
            println!();
        }
        output::render_terminal(&projects, cli.depth);
    }

    Ok(())
}
