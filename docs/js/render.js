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
  const rejectCount = payload.rejects?.length ?? 0;
  const stats = `<p><strong>${payload.date}</strong> · ${payload.accepts.length} accepts · ${rejectCount} rejects</p>`;
  const legend = `<p class="legend">Claim heat: 🟢 clean · 🟡 comments only · 🟠 assigned</p>`;
  const cards = payload.accepts.length
    ? payload.accepts.slice(0, 5).map(renderCard).join("")
    : `<p><em>No accepts.</em></p>`;
  mount.innerHTML = stats + legend + `<h2>Top candidates</h2>` + cards;
}

function escape(s) {
  return String(s).replace(/[&<>"']/g, c => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
  })[c]);
}
