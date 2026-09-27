use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;

use oss_issue_scout::payload::IssueMeta;
use oss_issue_scout::{config, evidence, github, grade, history, notify, payload};

fn main() -> Result<()> {
    let cfg = config::load("watched.yml")?;
    let today = Utc::now().date_naive();

    let mut packets: Vec<(String, String)> = Vec::new();
    let mut meta: HashMap<String, IssueMeta> = HashMap::new();
    for repo in &cfg.watched {
        eprintln!("fetching {repo}...");
        let facts = github::fetch_repo_facts(repo)?;
        for issue in github::fetch_open_issues(repo)? {
            let comments = if issue.comments > 0 {
                github::fetch_comments(repo, issue.number).unwrap_or_default()
            } else {
                vec![]
            };
            let text = evidence::build(repo, &facts, &issue, &comments);
            let url = issue.html_url.clone();
            meta.insert(url.clone(), IssueMeta {
                repo: repo.clone(),
                number: issue.number,
                title: issue.title.clone(),
                labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
                updated_at: issue.updated_at.clone(),
                comment_count: issue.comments,
                has_assignee: !issue.assignees.is_empty(),
                has_comments: issue.comments > 0,
            });
            packets.push((url, text));
        }
    }
    eprintln!("built {} evidence packets", packets.len());

    let cache = payload::cache_from_yesterday(today);
    let (cached, to_grade): (Vec<_>, Vec<_>) = packets.into_iter().partition(|(url, _)| {
        meta.get(url)
            .zip(cache.get(url))
            .is_some_and(|(m, (cached_updated, _))| &m.updated_at == cached_updated)
    });
    eprintln!("cache: {} hits, {} to grade", cached.len(), to_grade.len());

    let mut verdicts: Vec<grade::Verdict> = cached
        .into_iter()
        .filter_map(|(url, _)| cache.get(&url).map(|(_, v)| v.clone()))
        .collect();
    verdicts.extend(grade::grade_batch(&to_grade)?);

    let p = payload::build(today, &verdicts, &meta);
    payload::write(&p)?;
    payload::rebuild_manifest()?;

    if let Some(top) = p.accepts.first() {
        if history::is_new_top(today, &top.url) {
            notify::slack(&top.url)?;
        }
    }

    Ok(())
}
