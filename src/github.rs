use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const API: &str = "https://api.github.com";

fn token() -> Result<String> {
    std::env::var("GITHUB_TOKEN").map_err(|_| anyhow!("GITHUB_TOKEN not set"))
}

fn get<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T> {
    let tok = token()?;
    let resp = ureq::get(url)
        .set("Authorization", &format!("Bearer {tok}"))
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "oss-issue-scout")
        .call()?;
    Ok(resp.into_json()?)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepoFacts {
    pub full_name: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub stargazers_count: u32,
    pub pushed_at: String,
    #[serde(default)]
    pub language: Option<String>,
}

pub fn fetch_repo_facts(repo: &str) -> Result<RepoFacts> {
    get(&format!("{API}/repos/{repo}"))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Issue {
    pub number: u32,
    pub title: String,
    pub html_url: String,
    pub state: String,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub assignees: Vec<User>,
    pub user: User,
    #[serde(default)]
    pub author_association: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub comments: u32,
    #[serde(default)]
    pub body: Option<String>,
    // Present on PRs; used to filter them out of the issue list.
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub login: String,
}

pub fn fetch_open_issues(repo: &str) -> Result<Vec<Issue>> {
    let items: Vec<Issue> = get(&format!(
        "{API}/repos/{repo}/issues?state=open&per_page=100&sort=updated"
    ))?;
    Ok(items.into_iter().filter(|i| i.pull_request.is_none()).collect())
}

#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    pub user: User,
    #[serde(default)]
    pub author_association: String,
    pub created_at: String,
    #[serde(default)]
    pub body: Option<String>,
}

pub fn fetch_comments(repo: &str, number: u32) -> Result<Vec<Comment>> {
    get(&format!(
        "{API}/repos/{repo}/issues/{number}/comments?per_page=40"
    ))
}
