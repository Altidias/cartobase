const $ = (id) => document.getElementById(id);
let token = localStorage.getItem("cartobase_token") || "";

async function api(path, opts = {}) {
  const res = await fetch(path, {
    ...opts,
    headers: {
      "Authorization": "Bearer " + token,
      "Content-Type": "application/json",
      ...(opts.headers || {}),
    },
  });
  if (!res.ok) {
    let msg = "HTTP " + res.status;
    try { msg = (await res.json()).error || msg; } catch (_) {}
    throw new Error(msg);
  }
  return res.status === 204 ? null : res.json();
}

function setStatus(text, cls) {
  const el = $("status");
  el.textContent = text;
  el.className = "status " + (cls || "");
}

async function connect() {
  token = $("token").value.trim();
  if (!token) return;
  try {
    await refresh();
    localStorage.setItem("cartobase_token", token);
    $("panel").hidden = false;
    setStatus("connected", "ok");
  } catch (e) {
    $("panel").hidden = true;
    setStatus(e.message, "err");
  }
}

async function refresh() {
  const [stats, crews, tokens] = await Promise.all([
    api("/api/v1/admin/stats"),
    api("/api/v1/admin/crews"),
    api("/api/v1/admin/tokens"),
  ]);
  renderStats(stats);
  renderTokens(tokens, crews);
}

function card(n, l) {
  return `<div class="card"><div class="n">${n}</div><div class="l">${l}</div></div>`;
}

function renderStats(s) {
  $("stats").innerHTML =
    card(s.total_chunks.toLocaleString(), "chunks") +
    card(s.players, "players") +
    card(s.crews, "crews") +
    card(s.tokens, "tokens") +
    card(s.waypoints, "waypoints");
  const body = $("dims").querySelector("tbody");
  body.innerHTML = s.by_dimension
    .map((d) => `<tr><td>${esc(d.world)}</td><td>${esc(d.dimension)}</td><td>${d.category}</td><td>${d.chunks.toLocaleString()}</td></tr>`)
    .join("") || `<tr><td colspan="4" style="color:var(--muted)">no data yet</td></tr>`;
}

function renderTokens(tokens, crews) {
  const crewName = {};
  crews.forEach((c) => (crewName[c.id] = c.name));
  const body = $("tokens").querySelector("tbody");
  body.innerHTML = tokens
    .map((t) => {
      const state = t.revoked ? "revoked" : "active";
      const action = t.revoked ? "" : `<button class="link" data-revoke="${t.id}">revoke</button>`;
      return `<tr><td>${esc(t.player_name)}</td><td>${esc(crewName[t.crew_id] || "?")}</td><td>${t.role}</td><td>${state}</td><td>${action}</td></tr>`;
    })
    .join("");
  body.querySelectorAll("[data-revoke]").forEach((b) =>
    b.addEventListener("click", () => revoke(b.dataset.revoke))
  );
}

async function revoke(id) {
  try {
    await api(`/api/v1/admin/tokens/${id}/revoke`, { method: "POST" });
    await refresh();
  } catch (e) {
    setStatus(e.message, "err");
  }
}

async function mint(ev) {
  ev.preventDefault();
  const form = new FormData(ev.target);
  const payload = {
    crew: form.get("crew"),
    player_name: form.get("player_name"),
    role: form.get("role"),
  };
  try {
    const created = await api("/api/v1/admin/tokens", { method: "POST", body: JSON.stringify(payload) });
    const out = $("minted");
    out.hidden = false;
    out.textContent = `token for ${created.player_name} (${created.role}) — shown once:\n${created.token}`;
    await refresh();
  } catch (e) {
    setStatus(e.message, "err");
  }
}

function esc(s) {
  return String(s).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c]));
}

$("connect").addEventListener("click", connect);
$("token").addEventListener("keydown", (e) => e.key === "Enter" && connect());
$("mint").addEventListener("submit", mint);

if (token) {
  $("token").value = token;
  connect();
}
