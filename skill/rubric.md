# Rubric: is this a good first issue?

## Checks

| Check | Evidence | Pass condition | Weight |
|---|---|---|---|
| issue-open | Issue state header on the issue page (live), or the issue's `state` field in the eval bundle | Issue state is `open` (not `closed`) | required |
| no-open-linked-pr | Development box in the issue's right sidebar (live) and any PR references in the comment thread; in eval, the "linked PRs:" line under Repo facts plus PRs mentioned in the Comments section (see references/evidence-guide.md, Family 4) | No open PR is formally linked to the issue and no comment announces an in-progress PR that is still open | required |
| recent-default-branch-commits | "last 5 default-branch commits" line under Repo facts (eval), or the repo front page commit list (live). Measured against the capture date in eval, today in live. See evidence-guide.md, Family 1 | At least one commit on the default branch within the last 60 days, authored by a human (bots merging human PRs count) | required |
| maintainer-response-sample | "maintainer first-response sample" under Repo facts (eval), or recently-updated issues with Owner/Member/Collaborator badges on first replies (live). See Family 1 | At least one issue in the sample has a maintainer first response within 90 days | preferred |
| repo-not-archived | The `archived:` field on the repo line (eval) or the "This repository has been archived" banner (live). See Family 2 | Repo is not archived | required |
| repo-in-active-use | "latest release" and "last push to any branch" under Repo facts (eval), or the Releases sidebar and front-page commit date (live). See Family 2 | Latest release OR last push to any branch is within the last 180 days | required |
| bounded-newcomer-scope | The issue body and comment thread (eval Comments section, or the issue page live). See Family 3 | The issue describes one bounded piece of work: it is not labelled or described as an umbrella/tracking issue, the design is not still being actively debated by maintainers, and no maintainer has said the fix requires changes to core internals | required |
| not-support-request | The issue title and body | The issue is a bug report or a concrete change request, not a pure usage/how-do-I question | required |
| has-triage-label | Issue labels line (eval: `labels:` on the "opened by" line, or the issue's `labels` field; live: label chips on the issue page). See Family 3/4 | The issue has at least one label applied, AND none of its labels are known-negative signals: `duplicate`, `wontfix`, `won't fix`, `invalid`, `spam`, `not-a-bug`, `stale`, `question`. "labels: none" fails; a label set consisting only of negative labels fails | required |
| no-ai-contribution-ban | "contribution policy" line under Repo facts (eval) or `CONTRIBUTING.md`, `.github/CONTRIBUTING.md`, `AI_POLICY.md`, `AI_USAGE_POLICY.md`, and PR/issue templates (live). See "The fifth surface" in evidence-guide.md | The policy does not outright ban AI-generated contributions. Conditions like disclosure, testing, or human review are not bans and pass this check. Silence passes | required |
| good-first-issue-label | Labels on the issue (eval label list or live sidebar). See Family 4 | A `good first issue`, `good-first-issue`, or equivalent newcomer-friendly label is present and was added by a maintainer | preferred |
| no-abandoned-attempts | Linked PRs (eval "linked PRs:" line or live Development box) plus the comment thread for prior claim-and-drop history. See Family 4 | The issue has 0 or 1 closed unmerged PRs from prior contributors AND no more than 2 abandoned claim events (auto-unassignment by a bot, or a `working on this`/`I'll take this`/`@bot claim` comment with no follow-up PR after 90+ days) | required |

## Verdict rule

Accept only if every `required` check passes. `preferred` checks never change the verdict; they only rank the accepted issues. `unclear` on any `required` check counts as a fail.
