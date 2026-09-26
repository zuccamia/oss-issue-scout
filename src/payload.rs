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
pub struct Payload {
    pub date: NaiveDate,
    pub accepts: Vec<AcceptEntry>,
    pub reject_count: usize,
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
    accepts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let reject_count = verdicts.iter()
        .filter(|v| v.verdict.eq_ignore_ascii_case("reject"))
        .count();

    Payload { date, accepts, reject_count }
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
