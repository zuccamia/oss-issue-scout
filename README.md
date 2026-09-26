# oss-issue-scout

Daily grading of open-source issues against the [`issue-select`](skill/rubric.md) rubric, published as a static site.

## Flow

```
discover-daily (cron 09:00 UTC) → Slack proposals with promote links
    ↓ workflow_run
scout-daily → grades watched.yml issues, updates docs/, Slack on new top-1
    ↓ commit
GitHub Pages serves docs/
```

Click a Slack promote link → GitHub issue with `promote` label → `promote-to-watched` workflow appends to `watched.yml` → next day's scout picks it up.

## Setup

Requirements: `claude` CLI (locally, for testing), Rust toolchain.

Repo secrets:

| Secret | Source | Required |
|---|---|---|
| `CLAUDE_CODE_OAUTH_TOKEN` | `claude setup-token` | yes |
| `SLACK_WEBHOOK_URL` | Slack incoming webhook | no |

`GITHUB_TOKEN` is provided by Actions.

## Local run

```bash
export GITHUB_TOKEN=$(gh auth token)
cargo run --bin scout       # or --bin discover
```

## External API limits

| API | Limit | Impact |
|---|---|---|
| GitHub REST (Actions token) | 1000 req/hr per repo | ~2 calls per repo + 1 per issue |
| GitHub REST (personal token) | 5000 req/hr | Same volume, more headroom |
| GitHub search (discover) | 30 req/min authenticated | One search per language×topic pair |
| Claude subscription (OAuth) | Per-plan caps, ~4h refill | Serial grading with 20s pacing keeps runs under a single window |
| Slack webhook | 1 msg/sec | At most one per run |
