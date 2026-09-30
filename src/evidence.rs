use crate::github::{Comment, CommitSummary, Issue, LinkedPr, RepoFacts};
use chrono::Utc;

pub struct RepoContext {
    pub facts: RepoFacts,
    pub last_commits: Vec<CommitSummary>,
    pub contribution_policy: String,
}

// Build an eval-style bundle: everything the skill needs to grade in EVAL MODE
// without touching the network. Mirrors the format run_eval.py bundles use.
pub fn build(
    repo: &str,
    ctx: &RepoContext,
    issue: &Issue,
    comments: &[Comment],
    linked_prs: &[LinkedPr],
) -> String {
    let today = Utc::now().date_naive();
    let mut out = String::new();

    out.push_str(&format!("# Live item: {repo}#{}\n\n", issue.number));
    out.push_str(&format!("- source: {}\n", issue.html_url));
    out.push_str(&format!("- captured: {today}\n\n"));

    out.push_str(&format!("## Repo facts (captured {today})\n\n"));
    out.push_str(&format!(
        "- repo: {} ({} stars, archived: {})\n",
        ctx.facts.full_name, ctx.facts.stargazers_count, ctx.facts.archived
    ));
    if let Some(lang) = &ctx.facts.language {
        out.push_str(&format!("- primary language: {lang}\n"));
    }
    out.push_str(&format!("- last push to any branch: {}\n", ctx.facts.pushed_at));

    if ctx.last_commits.is_empty() {
        out.push_str("- last 5 default-branch commits: (none fetched)\n");
    } else {
        out.push_str("- last 5 default-branch commits:\n");
        for c in &ctx.last_commits {
            out.push_str(&format!(
                "  - {} by {}: {}\n",
                &c.date[..10.min(c.date.len())], c.author, c.message_line
            ));
        }
    }

    let linked_str = if linked_prs.is_empty() {
        "none".to_string()
    } else {
        linked_prs.iter()
            .map(|p| format!("#{} ({})", p.number, p.state))
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str(&format!(
        "- this issue: assignees: {}; linked PRs: {}\n",
        if issue.assignees.is_empty() {
            "none".to_string()
        } else {
            issue.assignees.iter().map(|u| u.login.clone()).collect::<Vec<_>>().join(", ")
        },
        linked_str,
    ));

    if ctx.contribution_policy.trim().is_empty() {
        out.push_str("- contribution policy: no CONTRIBUTING.md, AI_POLICY.md, or AGENTS.md found\n\n");
    } else {
        out.push_str("- contribution policy:\n");
        for line in ctx.contribution_policy.lines() {
            out.push_str(&format!("  {line}\n"));
        }
        out.push('\n');
    }

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
