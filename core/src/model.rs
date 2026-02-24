use chrono::{DateTime, Local};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Commit {
    pub hash: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_type: Option<String>,
    #[serde(rename = "timestamp")]
    pub time: DateTime<Local>,
    pub relative_time: String,
}

#[derive(Debug, Serialize)]
pub struct BranchLog {
    pub name: String,
    pub commits: Vec<Commit>,
}

#[derive(Debug, Serialize)]
pub struct ProjectLog {
    pub project: String,
    pub path: String,
    pub branches: Vec<BranchLog>,
}

impl BranchLog {
    pub fn latest_activity(&self) -> Option<&str> {
        self.commits.first().map(|c| c.relative_time.as_str())
    }
}

impl ProjectLog {
    pub fn total_commits(&self) -> usize {
        self.branches.iter().map(|b| b.commits.len()).sum()
    }

    pub fn latest_activity(&self) -> Option<&str> {
        self.branches
            .iter()
            .flat_map(|b| b.commits.first())
            .max_by_key(|c| c.time)
            .map(|c| c.relative_time.as_str())
    }
}
