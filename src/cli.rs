use crate::period::Period;
use clap::Parser;
use std::path::PathBuf;

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

    /// Output as JSON instead of colored terminal tree
    #[arg(long)]
    pub json: bool,

    /// Interactive drill-down mode (projects > branches > commits)
    #[arg(short, long, conflicts_with = "json")]
    pub interactive: bool,

    /// Filter by author name (defaults to git config user.name)
    #[arg(short, long)]
    pub author: Option<String>,
}
