use crate::github::{Comment, Issue, RepoFacts};
use chrono::Utc;

// Build an eval-style bundle: everything the skill needs to grade in EVAL MODE
// without touching the network. Mirrors the format run_eval.py bundles use.
pub fn build(repo: &str, facts: &RepoFacts, issue: &Issue, comments: &[Comment]) -> String {
    let today = Utc::now().date_naive();
    let mut out = String::new();

    out.push_str(&format!("# Live item: {repo}#{}\n\n", issue.number));
    out.push_str(&format!("- source: {}\n", issue.html_url));
    out.push_str(&format!("- captured: {today}\n\n"));

    out.push_str(&format!("## Repo facts (captured {today})\n\n"));
    out.push_str(&format!(
        "- repo: {} ({} stars, archived: {})\n",
        facts.full_name, facts.stargazers_count, facts.archived
    ));
    if let Some(lang) = &facts.language {
        out.push_str(&format!("- primary language: {lang}\n"));
    }
    out.push_str(&format!("- last push to any branch: {}\n", facts.pushed_at));
    out.push_str(&format!(
        "- this issue: assignees: {}; linked PRs: (not fetched)\n",
        if issue.assignees.is_empty() {
            "none".to_string()
        } else {
            issue.assignees.iter().map(|u| u.login.clone()).collect::<Vec<_>>().join(", ")
        }
    ));
    out.push_str("- contribution policy: (not fetched)\n\n");

    out.push_str("## Issue\n\n");
    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
    out.push_str(&format!("### {} (#{})\n\n", issue.title, issue.number));
    out.push_str(&format!(
        "opened by {} ({}) on {}, state {}, labels: {}\n\n",
        issue.user.login,
        issue.author_association,
        &issue.created_at[..10.min(issue.created_at.len())],
        issue.state,
        if labels.is_empty() { "none".to_string() } else { labels.join(", ") }
    ));
    if let Some(body) = &issue.body {
        out.push_str(body);
        out.push_str("\n\n");
    }

    out.push_str(&format!("## Comments ({} total)\n\n", issue.comments));
    for c in comments {
        out.push_str(&format!(
            "### {} ({}) on {}\n\n",
            c.user.login,
            c.author_association,
            &c.created_at[..10.min(c.created_at.len())]
        ));
        if let Some(b) = &c.body {
            out.push_str(b);
            out.push_str("\n\n");
        }
    }

    out
}
