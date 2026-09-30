// Shared card renderer. Consumed by home.js and archive.js.

const GH = "https://github.com/zuccamia/oss-issue-scout";

function relativeAge(iso) {
  const secs = Math.max(0, (Date.now() - new Date(iso).getTime()) / 1000);
  if (secs < 3600)      return "just now";
  if (secs < 86400)     return `${Math.floor(secs / 3600)}h ago`;
  if (secs < 2592000)   return `${Math.floor(secs / 86400)}d ago`;
  return `${Math.floor(secs / 2592000)}mo ago`;
}

function heat(a) {
  if (a.has_assignee) return "🟠";
  if (a.has_comments) return "🟡";
  return "🟢";
}

function shortRepo(fullName) {
  const i = fullName.lastIndexOf("/");
  return i >= 0 ? fullName.slice(i + 1) : fullName;
}

function renderCard(a) {
  const chips = a.labels.map(l => `<code>${escape(l)}</code>`).join(" ");
  const meta = `updated ${relativeAge(a.updated_at)} · ${a.comment_count} comment${a.comment_count === 1 ? "" : "s"}`;
  const metaLine = chips ? `${chips} · ${meta}` : meta;
  return `<article class="card">
    <header>${heat(a)} <strong><a target="_blank" rel="noopener noreferrer" href="${escape(a.url)}">${escape(shortRepo(a.repo))}#${a.number}</a></strong></header>
    <p class="title">${escape(a.title)}</p>
    <p class="meta">${metaLine}</p>
  </article>`;
}

function renderReport(payload, mount) {
  const rejects = payload.rejects ?? [];
  const stats = `<p><strong>${payload.date}</strong> · ${payload.accepts.length} accepts · ${rejects.length} rejects</p>`;
  const legend = `<p class="legend">Claim heat: 🟢 clean · 🟡 comments only · 🟠 assigned</p>`;
  const cards = payload.accepts.length
    ? payload.accepts.slice(0, 5).map(renderCard).join("")
    : `<p><em>No accepts.</em></p>`;
  mount.innerHTML = stats + legend + `<h2>Top candidates</h2>` + cards + renderRejects(rejects);
}

// Human-readable rendering of failed check names. Unknown checks fall through
// to their raw name so custom rubric checks still show up.
const FAILED_LABEL = {
  "no-open-linked-pr": "PR already open",
  "recent-default-branch-commits": "stale repo",
  "maintainer-response-sample": "slow to respond",
  "repo-not-archived": "archived",
  "repo-in-active-use": "inactive",
  "bounded-newcomer-scope": "unclear scope",
  "not-support-request": "support question",
  "has-triage-label": "untriaged",
  "no-ai-contribution-ban": "AI banned",
  "good-first-issue-label": "no gfi label",
  "no-abandoned-attempts": "prior attempts",
};

function checkLabel(name) {
  return FAILED_LABEL[name] || name;
}

function renderRejects(rejects) {
  if (!rejects.length) return "";

  // Aggregate: count how often each check fails.
  const counts = new Map();
  rejects.forEach(r => (r.failed_checks || []).forEach(name => {
    counts.set(name, (counts.get(name) || 0) + 1);
  }));
  const topFailed = [...counts.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5)
    .map(([name, n]) => `<code title="${escape(name)}">${escape(checkLabel(name))}</code> (${n})`)
    .join(" ");

  // Recent rejects: sort by updated_at desc, take 10.
  const recent = [...rejects]
    .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
    .slice(0, 10);

  const rows = recent.map(r => {
    const m = r.url.match(/github\.com\/[^/]+\/([^/]+)\/issues\/(\d+)/);
    const label = m ? `${m[1]}#${m[2]}` : r.url;
    const chips = (r.failed_checks || []).map(c => `<code title="${escape(c)}">${escape(checkLabel(c))}</code>`).join(" ");
    return `<li><a target="_blank" rel="noopener noreferrer" href="${escape(r.url)}">${escape(label)}</a> · ${chips}</li>`;
  }).join("");

  return `<details class="rejects">
    <summary>Rejects (${rejects.length}) — top reasons: ${topFailed || "none"}</summary>
    <ul class="rejects-list">${rows}</ul>
  </details>`;
}

function escape(s) {
  return String(s).replace(/[&<>"']/g, c => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
  })[c]);
}
