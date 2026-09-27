use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::thread;
use std::time::Duration as StdDuration;

// GitHub search allows 30 authenticated req/min. 2.5s spacing = 24/min, safe headroom.
const SEARCH_DELAY: StdDuration = StdDuration::from_millis(2500);

// Slots per language in the final shortlist, and total shortlist size.
const PER_LANGUAGE_CAP: usize = 3;
const TOTAL_SLOTS: usize = 10;

// Recency half-life in days: pushed today = 1.0, ~30d ago = 0.5, ~90d ago = 0.13.
fn recency_score(pushed_at: Option<&String>) -> f64 {
    let Some(iso) = pushed_at else { return 0.0 };
    let Ok(t) = DateTime::parse_from_rfc3339(iso) else { return 0.0 };
    let age_days = (Utc::now() - t.with_timezone(&Utc)).num_days().max(0) as f64;
    (-age_days / 30.0_f64.ln() * (2.0_f64.ln() / 30.0)).exp()
        .clamp(0.0, 1.0)
}

// Composite score, dampened star popularity + multi-topic hit bonus + recency.
fn score(repo: &Repo, hit_count: u32) -> f64 {
    let star_pop = ((repo.stargazers_count as f64) + 1.0).log10();  // 100★ = 2.0, 10k★ = 4.0
    let hit_bonus = (hit_count as f64 - 1.0).max(0.0) * 0.5;
    let recency = recency_score(repo.pushed_at.as_ref());
    star_pop * 0.4 + hit_bonus + recency
}

// Bucket by language, sort each bucket by score, then interleave: pick 1 from each
// language round-robin until slots fill. Prevents Python from eating the shortlist.
fn rank_and_allocate(
    hits: std::collections::HashMap<String, (Repo, u32)>,
    per_lang_cap: usize,
    total: usize,
) -> Vec<(Repo, u32)> {
    use std::collections::HashMap;
    let mut per_lang: HashMap<String, Vec<(Repo, u32, f64)>> = HashMap::new();
    for (_, (repo, h)) in hits {
        let s = score(&repo, h);
        let lang = repo.language.clone().unwrap_or_else(|| "_other".into());
        per_lang.entry(lang).or_default().push((repo, h, s));
    }
    for v in per_lang.values_mut() {
        v.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(per_lang_cap);
    }
    let mut out: Vec<(Repo, u32, f64)> = per_lang.into_values().flatten().collect();
    out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(total);
    out.into_iter().map(|(r, h, _)| (r, h)).collect()
}

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

#[derive(Debug, Clone, Deserialize)]
struct Repo {
    full_name: String,
    stargazers_count: u32,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    open_issues_count: u32,
    #[serde(default)]
    pushed_at: Option<String>,
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

    let qualifiers: Vec<String> = if d.criteria.newcomer_qualifiers.is_empty() {
        vec![String::new()]
    } else {
        d.criteria.newcomer_qualifiers.clone()
    };

    // full_name -> (repo, hit_count). Hit count = distinct (lang, topic) combos
    // that returned this repo. A repo tagged multiple relevant topics gets a bonus.
    let mut hits: std::collections::HashMap<String, (Repo, u32)> = Default::default();

    for lang in &d.criteria.languages {
        for topic in &d.criteria.topics_any {
            let mut combo_seen: HashSet<String> = HashSet::new();
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
                    if skip.contains(&repo.full_name) { continue; }
                    // Count this repo at most once per (lang, topic), even if it
                    // appears under both newcomer qualifiers.
                    if !combo_seen.insert(repo.full_name.clone()) { continue; }
                    hits.entry(repo.full_name.clone())
                        .and_modify(|(_, h)| *h += 1)
                        .or_insert((repo, 1));
                }
            }
        }
    }

    let proposals = rank_and_allocate(hits, PER_LANGUAGE_CAP, TOTAL_SLOTS);

    if proposals.is_empty() {
        eprintln!("no new candidates.");
        slack_notify("🔍 scout discovery: no new candidates today.")?;
        return Ok(());
    }

    let mut msg = format!("🔍 scout discovery — {} new candidate(s)\n", proposals.len());
    for (i, (r, hits)) in proposals.iter().enumerate() {
        let desc = r.description.as_deref().unwrap_or("").chars().take(90).collect::<String>();
        let lang = r.language.as_deref().unwrap_or("-");
        let pushed = r.pushed_at.as_deref().map(|s| &s[..10.min(s.len())]).unwrap_or("?");
        let hits_note = if *hits > 1 { format!(" · {hits}× topic hits") } else { String::new() };
        msg.push_str(&format!(
            "\n{}. <https://github.com/{}|*{}*> — {} · {}★ · {} open issues · pushed {}{}\n   _{}_ | <{}|watch>\n",
            i + 1, r.full_name, r.full_name, lang, r.stargazers_count, r.open_issues_count, pushed, hits_note, desc,
            promote_link(&scout_repo, &r.full_name)
        ));
    }

    slack_notify(&msg)?;
    println!("{msg}");
    Ok(())
}
