use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::grade::Verdict;

const DATA_DIR: &str = "docs/data";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptEntry {
    pub url: String,
    pub repo: String,
    pub number: u32,
    pub title: String,
    pub labels: Vec<String>,
    pub updated_at: String,
    pub comment_count: u32,
    pub has_assignee: bool,
    pub has_comments: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectRef {
    pub url: String,
    pub updated_at: String,
    #[serde(default)]
    pub failed_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub date: NaiveDate,
    pub accepts: Vec<AcceptEntry>,
    #[serde(default)]
    pub rejects: Vec<RejectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateSummary {
    pub date: NaiveDate,
    pub accept_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub dates: Vec<DateSummary>,
}

// Enriched card fields per URL, gathered by scout from GitHub API responses.
#[derive(Clone)]
pub struct IssueMeta {
    pub repo: String,
    pub number: u32,
    pub title: String,
    pub labels: Vec<String>,
    pub updated_at: String,
    pub comment_count: u32,
    pub has_assignee: bool,
    pub has_comments: bool,
}

fn failed_check_names(checks: &[serde_json::Value]) -> Vec<String> {
    checks.iter()
        .filter_map(|c| {
            let obj = c.as_object()?;
            let grade = obj.get("grade")?.as_str()?;
            if grade.eq_ignore_ascii_case("pass") { return None; }
            obj.get("name")?.as_str().map(String::from)
        })
        .collect()
}

pub fn build(
    date: NaiveDate,
    verdicts: &[Verdict],
    meta: &HashMap<String, IssueMeta>,
) -> Payload {
    let mut accepts: Vec<AcceptEntry> = verdicts
        .iter()
        .filter(|v| v.verdict.eq_ignore_ascii_case("accept"))
        .filter_map(|v| meta.get(&v.item).map(|m| AcceptEntry {
            url: v.item.clone(),
            repo: m.repo.clone(),
            number: m.number,
            title: m.title.clone(),
            labels: m.labels.clone(),
            updated_at: m.updated_at.clone(),
            comment_count: m.comment_count,
            has_assignee: m.has_assignee,
            has_comments: m.has_comments,
            note: v.note.clone(),
        }))
        .collect();
    accepts.sort_by_key(|a| std::cmp::Reverse(a.updated_at.clone()));

    let rejects: Vec<RejectRef> = verdicts
        .iter()
        .filter(|v| v.verdict.eq_ignore_ascii_case("reject"))
        .filter_map(|v| meta.get(&v.item).map(|m| RejectRef {
            url: v.item.clone(),
            updated_at: m.updated_at.clone(),
            failed_checks: failed_check_names(&v.checks),
        }))
        .collect();

    Payload { date, accepts, rejects }
}

pub fn write(payload: &Payload) -> Result<()> {
    fs::create_dir_all(DATA_DIR)?;
    fs::write(
        format!("{DATA_DIR}/{}.json", payload.date),
        serde_json::to_string_pretty(payload)?,
    )?;
    Ok(())
}

pub fn rebuild_manifest() -> Result<()> {
    fs::create_dir_all(DATA_DIR)?;
    let mut dates: Vec<DateSummary> = Vec::new();
    if Path::new(DATA_DIR).exists() {
        for entry in fs::read_dir(DATA_DIR)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(".json") else { continue };
            if stem == "manifest" { continue; }
            let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") else { continue };
            let text = fs::read_to_string(entry.path())?;
            let p: Payload = serde_json::from_str(&text)?;
            dates.push(DateSummary {
                date,
                accept_count: p.accepts.len(),
                top_url: p.accepts.first().map(|a| a.url.clone()),
            });
        }
    }
    dates.sort_by_key(|d| std::cmp::Reverse(d.date));
    let m = Manifest { dates };
    fs::write(
        format!("{DATA_DIR}/manifest.json"),
        serde_json::to_string_pretty(&m)?,
    )?;
    Ok(())
}

pub fn load(date: NaiveDate) -> Result<Option<Payload>> {
    let path = format!("{DATA_DIR}/{date}.json");
    if !Path::new(&path).exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
}

// Cache lookup keyed by url. Value is (updated_at_when_last_graded, prior_Verdict).
// If today's issue's updated_at matches the cached one, reuse the verdict.
pub type Cache = HashMap<String, (String, Verdict)>;

pub fn cache_from_yesterday(today: NaiveDate) -> Cache {
    let Some(y) = today.pred_opt() else { return HashMap::new() };
    let Ok(Some(p)) = load(y) else { return HashMap::new() };
    let mut c = HashMap::new();
    for a in p.accepts {
        c.insert(a.url.clone(), (a.updated_at, Verdict {
            item: a.url,
            checks: vec![],
            verdict: "accept".into(),
            note: a.note,
        }));
    }
    for r in p.rejects {
        let checks = r.failed_checks.iter()
            .map(|name| serde_json::json!({ "name": name, "grade": "fail" }))
            .collect();
        c.insert(r.url.clone(), (r.updated_at, Verdict {
            item: r.url,
            checks,
            verdict: "reject".into(),
            note: None,
        }));
    }
    c
}
