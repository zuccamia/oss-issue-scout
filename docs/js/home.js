// Fetch manifest → most recent date → that day's payload → render.

const mount = document.getElementById("report");

fetch("data/manifest.json")
  .then(r => r.ok ? r.json() : Promise.reject(r.status))
  .then(m => {
    const latest = m.dates?.[0]?.date;
    if (!latest) throw new Error("empty manifest");
    return fetch(`data/${latest}.json`);
  })
  .then(r => r.ok ? r.json() : Promise.reject(r.status))
  .then(p => renderReport(p, mount))
  .catch(err => {
    mount.innerHTML = `<p><em>No report yet. First run pending.</em></p>`;
    console.warn(err);
  });
