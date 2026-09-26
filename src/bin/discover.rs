use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::thread;
use std::time::Duration as StdDuration;

// GitHub search allows 30 authenticated req/min. 2.5s spacing = 24/min, safe headroom.
const SEARCH_DELAY: StdDuration = StdDuration::from_millis(2500);

use oss_issue_scout::config;

#[derive(Debug, Deserialize)]
struct Discover {
    criteria: Criteria,
    #[serde(default)]
    rejected: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Criteria {
    languages: Vec<String>,
    topics_any: Vec<String>,
    stars_min: u32,
    pushed_within_days: i64,
    #[serde(default)]
    newcomer_qualifiers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    items: Vec<Repo>,
}

#[derive(Debug, Deserialize)]
struct Repo {
    full_name: String,
    stargazers_count: u32,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    open_issues_count: u32,
}

fn token() -> Result<String> {
    std::env::var("GITHUB_TOKEN").map_err(|_| anyhow!("GITHUB_TOKEN not set"))
}

fn search(q: &str) -> Result<SearchResult> {
    let tok = token()?;
    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page=20",
        urlencoding_encode(q)
    );
    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {tok}"))
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "oss-issue-scout-discover")
        .call()?;
    Ok(resp.into_json()?)
}

// Minimal percent-encoder for the parts we use; the full url crate is overkill here.
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            _ => {
                let mut buf = [0u8; 4];
                for &b in c.encode_utf8(&mut buf).as_bytes() {
                    out.push_str(&format!("%{b:02X}"));
                }
            }
        }
    }
    out
}

fn promote_link(repo_url_slug: &str, repo: &str) -> String {
    let title = urlencoding_encode(&format!("[promote] {repo}"));
    let body = urlencoding_encode(&format!("repo: {repo}"));
    format!("https://github.com/{repo_url_slug}/issues/new?title={title}&body={body}&labels=promote")
}

fn slack_notify(text: &str) -> Result<()> {
    let Ok(webhook) = std::env::var("SLACK_WEBHOOK_URL") else { return Ok(()); };
    let _ = ureq::post(&webhook).send_json(json!({ "text": text }));
    Ok(())
}

fn main() -> Result<()> {
    let d: Discover = serde_yaml::from_str(&fs::read_to_string("repo-criteria.yml")?)?;
    let watched = config::load("watched.yml")?.watched;

    let scout_repo = std::env::var("GITHUB_REPOSITORY")
        .unwrap_or_else(|_| "OWNER/REPO".to_string());

    let mut skip: HashSet<String> = watched.into_iter().collect();
    skip.extend(d.rejected.iter().cloned());

    let pushed_since = (Utc::now() - Duration::days(d.criteria.pushed_within_days))
        .format("%Y-%m-%d")
        .to_string();

    let mut proposals: Vec<Repo> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let qualifiers: Vec<String> = if d.criteria.newcomer_qualifiers.is_empty() {
        vec![String::new()]
    } else {
        d.criteria.newcomer_qualifiers.clone()
    };

    for lang in &d.criteria.languages {
        for topic in &d.criteria.topics_any {
            for qual in &qualifiers {
                let mut q = format!(
                    "language:{lang} topic:{topic} stars:>={} pushed:>{}",
                    d.criteria.stars_min, pushed_since
                );
                if !qual.is_empty() {
                    q.push(' ');
                    q.push_str(qual);
                }
                q.push_str(" archived:false fork:false");
                eprintln!("search: {q}");
                thread::sleep(SEARCH_DELAY);
                let r = match search(&q) {
                    Ok(r) => r,
                    Err(e) => { eprintln!("  search failed: {e:#}"); continue; }
                };
                for repo in r.items {
                    if skip.contains(&repo.full_name) || !seen.insert(repo.full_name.clone()) {
                        continue;
                    }
                    proposals.push(repo);
                }
            }
        }
    }

    proposals.sort_by_key(|r| std::cmp::Reverse(r.stargazers_count));
    proposals.truncate(10);

    if proposals.is_empty() {
        eprintln!("no new candidates.");
        slack_notify("🔍 scout discovery: no new candidates today.")?;
        return Ok(());
    }

    let mut msg = format!("🔍 scout discovery — {} new candidate(s)\n", proposals.len());
    for (i, r) in proposals.iter().enumerate() {
        let desc = r.description.as_deref().unwrap_or("").chars().take(90).collect::<String>();
        let lang = r.language.as_deref().unwrap_or("-");
        msg.push_str(&format!(
            "\n{}. *{}* — {} · {}★ · {} open issues\n   _{}_\n   <{}|promote to watched>\n",
            i + 1, r.full_name, lang, r.stargazers_count, r.open_issues_count, desc,
            promote_link(&scout_repo, &r.full_name)
        ));
    }

    slack_notify(&msg)?;
    println!("{msg}");
    Ok(())
}
