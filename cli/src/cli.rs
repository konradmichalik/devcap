use worklog_core::period::Period;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Depth {
    /// Show only project names with summary
    Projects,
    /// Show projects and branches
    Branches,
    /// Show projects, branches, and commits (default)
    Commits,
}

#[derive(Parser, Debug)]
#[command(
    name = "worklog.git",
    about = "Aggregate git commits across repos for standups and time tracking",
    version
)]
pub struct Cli {
    /// Time period: today, yesterday, 24h, 3d, 7d, week
    #[arg(short, long, default_value = "today")]
    pub period: Period,

    /// Root directory to scan for git repos
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Enable colored output for commit types and markers
    #[arg(short, long)]
    pub color: bool,

    /// Interactive drill-down mode (projects > branches > commits)
    #[arg(short, long, conflicts_with = "json")]
    pub interactive: bool,

    /// Output depth: projects, branches, commits
    #[arg(short, long, default_value = "commits", conflicts_with = "json")]
    pub depth: Depth,

    /// Filter by author name (defaults to git config user.name)
    #[arg(short, long)]
    pub author: Option<String>,
}
