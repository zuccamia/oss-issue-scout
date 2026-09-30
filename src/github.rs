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

// One entry per commit: date + author name + first line of message.
#[derive(Debug, Clone)]
pub struct CommitSummary {
    pub date: String,
    pub author: String,
    pub message_line: String,
}

pub fn fetch_last_commits(repo: &str) -> Result<Vec<CommitSummary>> {
    #[derive(Deserialize)] struct RawCommit { commit: CommitObj }
    #[derive(Deserialize)] struct CommitObj { author: CommitAuthor, message: String }
    #[derive(Deserialize)] struct CommitAuthor { name: String, date: String }

    let raw: Vec<RawCommit> = get(&format!("{API}/repos/{repo}/commits?per_page=5"))?;
    Ok(raw.into_iter().map(|r| CommitSummary {
        date: r.commit.author.date,
        author: r.commit.author.name,
        message_line: r.commit.message.lines().next().unwrap_or("").to_string(),
    }).collect())
}

// Best-effort fetch of contribution/AI policy excerpts. Returns concatenated
// first-N-char snippets from each file that exists.
pub fn fetch_contribution_policy(repo: &str) -> String {
    const PATHS: &[&str] = &["CONTRIBUTING.md", ".github/CONTRIBUTING.md", "AI_POLICY.md", "AGENTS.md"];
    const SNIPPET: usize = 300;
    let mut out = String::new();
    for path in PATHS {
        if let Ok(text) = fetch_file(repo, path) {
            let excerpt: String = text.chars().take(SNIPPET).collect();
            out.push_str(&format!("{path}: {excerpt}\n"));
        }
    }
    out
}

fn fetch_file(repo: &str, path: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct FileResp { content: String, encoding: String }
    let f: FileResp = get(&format!("{API}/repos/{repo}/contents/{path}"))?;
    if f.encoding != "base64" {
        return Err(anyhow!("unexpected encoding: {}", f.encoding));
    }
    use base64::{engine::general_purpose, Engine as _};
    let stripped: String = f.content.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = general_purpose::STANDARD.decode(stripped)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

// Linked PRs on an issue, extracted from the timeline events.
#[derive(Debug, Clone)]
pub struct LinkedPr {
    pub number: u32,
    pub state: String,
}

pub fn fetch_linked_prs(repo: &str, number: u32) -> Result<Vec<LinkedPr>> {
    #[derive(Deserialize)]
    struct TimelineEvent {
        event: String,
        #[serde(default)] source: Option<Source>,
    }
    #[derive(Deserialize)]
    struct Source { issue: Option<SourceIssue> }
    #[derive(Deserialize)]
    struct SourceIssue {
        number: u32,
        state: String,
        #[serde(default)] pull_request: Option<serde_json::Value>,
    }

    let events: Vec<TimelineEvent> = get(&format!(
        "{API}/repos/{repo}/issues/{number}/timeline?per_page=100"
    ))?;
    let mut out = Vec::new();
    for e in events {
        if e.event != "cross-referenced" && e.event != "connected" { continue; }
        let Some(src) = e.source.and_then(|s| s.issue) else { continue };
        if src.pull_request.is_none() { continue; }
        out.push(LinkedPr { number: src.number, state: src.state });
    }
    Ok(out)
}
