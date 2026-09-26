// URL hash drives what shows: no hash = list, #YYYY-MM-DD = that day's report.

const mount = document.getElementById("report");

function showList(manifest) {
  if (!manifest.dates.length) {
    mount.innerHTML = `<p><em>No archived days yet.</em></p>`;
    return;
  }
  const rows = manifest.dates.map(d => {
    const top = d.top_url
      ? `top: <a target="_blank" rel="noopener noreferrer" href="${escape(d.top_url)}">${escape(d.top_url)}</a>`
      : `no accepts`;
    return `<li><a href="#${d.date}">${d.date}</a> · ${d.accept_count} accepts · ${top}</li>`;
  }).join("");
  mount.innerHTML = `<h2>Archive</h2><ul class="archive">${rows}</ul>`;
}

function showDay(date) {
  fetch(`../data/${date}.json`)
    .then(r => r.ok ? r.json() : Promise.reject(r.status))
    .then(p => {
      mount.innerHTML = `<p><a href="#">← back to archive</a></p>`;
      const cardWrap = document.createElement("div");
      renderReport(p, cardWrap);
      mount.appendChild(cardWrap);
    })
    .catch(err => {
      mount.innerHTML = `<p><em>Payload for ${escape(date)} not found.</em></p>
        <p><a href="#">← back to archive</a></p>`;
      console.warn(err);
    });
}

function route() {
  const hash = location.hash.replace(/^#/, "");
  if (/^\d{4}-\d{2}-\d{2}$/.test(hash)) {
    showDay(hash);
  } else {
    fetch("../data/manifest.json")
      .then(r => r.ok ? r.json() : Promise.reject(r.status))
      .then(showList)
      .catch(err => {
        mount.innerHTML = `<p><em>Manifest not found.</em></p>`;
        console.warn(err);
      });
  }
}

window.addEventListener("hashchange", route);
route();
