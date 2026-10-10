// Vortux installer window. Plain JavaScript on purpose: no build step.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (sel) => document.querySelector(sel);
const NAMES = { player: "Vortux Player", studio: "Vortux Studio" };
const LINKS = { player: "https://playvortex.io/download/windows", studio: "https://playvortex.io/download/studio-windows" };
const LAYOUTS = ["pyramid", "vortex", "v", "ring"];
const RAIL = { welcome: 0, check: 1, apps: 2, manual: 2, install: 3, done: 4 };

let mode = "install";
let picked = [];
let installed = [];
let manualId = null;
let scene = null;
let frameless = false;
let steps = 0, frac = 0, total = 8;
let current = "welcome", backTo = "welcome";

function el(tag, cls, text) {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text !== undefined) n.textContent = text;
  return n;
}

function show(name) {
  current = name;
  document.querySelectorAll(".screen, .stage").forEach((s) => { s.hidden = s.id !== "s-" + name; });
  document.body.dataset.view = name;
  const idx = RAIL[name] ?? -1;
  document.querySelectorAll("#rail li").forEach((li, i) => {
    li.classList.toggle("on", i === idx);
    li.classList.toggle("past", idx >= 0 && i < idx);
    if (i === idx) li.setAttribute("aria-current", "step"); else li.removeAttribute("aria-current");
  });
  const h = document.querySelector("#s-" + name + " h1, #s-" + name + " h2");
  if (h) h.focus({ preventScroll: true });
  const wantFrameless = name === "install";
  if (wantFrameless !== frameless) {
    frameless = wantFrameless;
    invoke("set_frameless", { on: frameless }).catch(() => {});
  }
  if (scene) scene.setMode(wantFrameless ? "build" : "float");
}

function showError(err) {
  $("#error-text").textContent = String(err);
  show("error");
}

function openButtons(box, ids) {
  box.replaceChildren();
  ids.forEach((id) => {
    const b = el("button", "btn primary", "Open " + NAMES[id]);
    b.addEventListener("click", () => invoke("launch_app", { id }).catch(showError));
    box.append(b);
  });
}

// ---------- community pictures: hero strip + 3D cubes ----------
async function loadCommunity() {
  let files = [];
  try {
    const r = await fetch("assets/community/manifest.json");
    if (r.ok) files = (await r.json()).images || [];
  } catch (e) { /* no pictures: the UI still works */ }

  const track = $("#strip");
  [...files, ...files].forEach((f) => {
    const img = el("img");
    img.src = "assets/community/" + f;
    img.alt = "";
    track.append(img);
  });

  try {
    const mod = await import("./scene.js");
    scene = await mod.startScene($("#scene"), files.map((f) => "assets/community/" + f.replace(/^g/, "t")));
    scene.setMode(frameless ? "build" : "float");
    applySettings();
  } catch (e) {
    document.body.classList.add("no3d"); // three.js missing or WebGL unavailable
  }
}

// Tilt cards toward the pointer.
document.querySelectorAll(".tilt3d").forEach((n) => {
  n.addEventListener("pointermove", (e) => {
    const r = n.getBoundingClientRect();
    n.style.setProperty("--ry", ((e.clientX - r.left) / r.width - 0.5) * 14 + "deg");
    n.style.setProperty("--rx", -((e.clientY - r.top) / r.height - 0.5) * 14 + "deg");
  });
  n.addEventListener("pointerleave", () => {
    n.style.setProperty("--rx", "0deg");
    n.style.setProperty("--ry", "0deg");
  });
});

// ---------- welcome / manager ----------
async function home() {
  let st = { player: false, studio: false };
  try { st = await invoke("install_status"); } catch (e) { /* treat as not installed */ }
  const have = Object.keys(NAMES).filter((id) => st[id]);
  $("#installed").hidden = have.length === 0;
  $("#installed-list").textContent = have.map((id) => NAMES[id]).join(", ");
  openButtons($("#installed-open"), have);
  $("#uninstall-ask").hidden = true;
  $("#uninstall").hidden = false;
  $("#go").textContent = have.length ? "Reinstall or add apps" : "Get started";
  if (scene) scene.setProgress(0);
  show("welcome");
}

// ---------- system check ----------
function row(name, kind, detail) {
  const li = el("li", kind);
  li.append(el("b", "", kind === "ok" ? "\u2713" : kind === "warn" ? "!" : "\u2715"), el("span", "name", name), el("span", "muted", detail));
  return li;
}

async function check() {
  show("check");
  $("#check-next").disabled = true;
  $("#problems").hidden = true;
  $("#checks").replaceChildren(row("Checking\u2026", "warn", ""));
  let i;
  try { i = await invoke("system_check"); } catch (e) { return showError(e); }
  const vk = i.vulkan === "Working" ? ["ok", "working"] : i.vulkan === "Unknown" ? ["warn", "couldn't check"] : ["bad", "not working"];
  $("#checks").replaceChildren(
    row("Wine", i.wine ? "ok" : "bad", i.wine || "not found"),
    row("Vulkan", vk[0], vk[1]),
    row("xdg-utils", i.xdg_mime ? "ok" : "bad", i.xdg_mime ? "found" : "not found"),
    row("winetricks", i.winetricks ? "ok" : "warn", i.winetricks ? "found" : "not found (needed for graphics components)")
  );
  const box = $("#problems");
  box.replaceChildren(...i.problems.map((p) => el("p", "", p)));
  if (i.problems.length) box.append(el("p", "muted", "Install the missing packages with your package manager, then check again."));
  box.hidden = i.problems.length === 0;
  $("#check-next").disabled = i.problems.length > 0;
}

// ---------- install ----------
function paint(label) {
  const p = Math.min(0.97, (Math.max(steps - 1, 0) + frac) / total);
  const pct = Math.round(p * 100);
  $("#fill").style.width = pct + "%";
  $("#bar").setAttribute("aria-valuenow", pct);
  $("#pct").textContent = label || pct + "%";
  if (scene) scene.setProgress(p);
}

function begin(title, count) {
  steps = 0; frac = 0; total = 5 + 3 * count;
  $("#h-install").textContent = title;
  $("#status").textContent = "Starting\u2026";
  $("#cancel").disabled = false;
  if (scene) scene.setLayout(new URLSearchParams(location.search).get("scene") || LAYOUTS[Math.floor(Math.random() * LAYOUTS.length)]);
  paint();
  show("install");
}

function showDone(text, title) {
  $("#h-done").textContent = title || "You're all set";
  $("#done-text").textContent = text || "Press play on the Vortex website, or open an app from here or your app menu.";
  openButtons($("#done-open"), installed);
  show("done");
}

function finish(r) {
  if (r.cancelled) {
    $("#apps-note").textContent = "Install cancelled. You can try again whenever you like.";
    $("#apps-note").hidden = false;
    return show("apps");
  }
  if (!r.ok) return showError(r.error || "Unknown error");
  if (mode === "uninstall") {
    installed = [];
    return showDone("The apps, the Wine environment and the menu entries are gone. You can close this window.", "Vortux has been removed");
  }
  const skipped = r.skipped || [];
  installed = [...new Set([...installed, ...picked.filter((id) => !skipped.includes(id))])];
  if (scene) scene.setProgress(1);
  if (skipped.length) {
    manualId = skipped[0];
    $("#manual-name").textContent = NAMES[manualId];
    $("#manual-link").textContent = LINKS[manualId];
    return show("manual");
  }
  showDone();
}

listen("vortux-step", (e) => { steps++; frac = 0; $("#status").textContent = e.payload; paint(); });
listen("vortux-warn", (e) => { $("#status").textContent = e.payload; });
listen("vortux-download", (e) => {
  const p = e.payload;
  $("#status").textContent = "Downloading " + p.name;
  if (p.total) {
    frac = p.done / p.total;
    paint(Math.floor(frac * 100) + "%  \u00b7  " + Math.round(p.done / 1e6) + " of " + Math.round(p.total / 1e6) + " MB");
  }
});
listen("vortux-done", (e) => finish(e.payload));

// Dropping a file on the "manual download" screen installs it.
listen("tauri://drag-enter", () => { if (!$("#s-manual").hidden) $("#drop").classList.add("over"); });
listen("tauri://drag-leave", () => $("#drop").classList.remove("over"));
listen("tauri://drag-drop", (e) => {
  $("#drop").classList.remove("over");
  const path = e.payload && e.payload.paths && e.payload.paths[0];
  if ($("#s-manual").hidden || !path) return;
  mode = "install";
  picked = [manualId];
  begin("Installing", 1);
  invoke("install_file", { id: manualId, path }).catch(showError);
});

// ---------- buttons ----------
$("#go").addEventListener("click", check);
$("#recheck").addEventListener("click", check);
$("#check-next").addEventListener("click", () => { $("#apps-note").hidden = true; show("apps"); });
$("#apps-back").addEventListener("click", () => show("check"));

document.querySelectorAll(".choice input").forEach((box) => {
  box.addEventListener("change", () => box.closest(".choice").classList.toggle("on", box.checked));
});

async function startInstall(ids) {
  mode = "install";
  picked = ids;
  begin("Installing", ids.length);
  try { await invoke("start_install", { apps: ids }); } catch (e) { showError(e); }
}

$("#install").addEventListener("click", () => {
  const ids = [...document.querySelectorAll(".choice input:checked")].map((b) => b.value);
  if (!ids.length) {
    $("#apps-note").textContent = "Pick at least one app to install.";
    $("#apps-note").hidden = false;
    return;
  }
  installed = [];
  startInstall(ids);
});

$("#manual-skip").addEventListener("click", () => showDone());
$("#manual-copy").addEventListener("click", async () => {
  const b = $("#manual-copy");
  try {
    await navigator.clipboard.writeText($("#manual-link").textContent);
    b.textContent = "Copied";
  } catch (e) {
    b.textContent = "Select the link to copy it";
  }
  setTimeout(() => { b.textContent = "Copy link"; }, 2000);
});

$("#cancel").addEventListener("click", () => {
  $("#cancel").disabled = true;
  $("#status").textContent = "Cancelling\u2026";
  invoke("cancel_install");
});

$("#uninstall").addEventListener("click", () => { $("#uninstall").hidden = true; $("#uninstall-ask").hidden = false; });
$("#uninstall-no").addEventListener("click", () => { $("#uninstall").hidden = false; $("#uninstall-ask").hidden = true; });
$("#uninstall-yes").addEventListener("click", async () => {
  mode = "uninstall";
  begin("Removing Vortux", 0);
  $("#cancel").disabled = true;
  try { await invoke("start_uninstall"); } catch (e) { showError(e); }
});

$("#done-home").addEventListener("click", home);
$("#error-back").addEventListener("click", home);

// ---------- settings ----------
const SETTINGS_KEY = "vortux.settings";
const defaults = { scene3d: true, motion: !window.matchMedia("(prefers-reduced-motion: reduce)").matches };
let settings = { ...defaults };
try {
  settings = { ...defaults, ...JSON.parse(localStorage.getItem(SETTINGS_KEY) || "{}") };
} catch (e) { /* use the defaults */ }

function applySettings() {
  document.body.classList.toggle("off3d", !settings.scene3d);
  document.body.classList.toggle("calm", !settings.motion);
  if (scene) {
    scene.setEnabled(settings.scene3d);
    scene.setCalm(!settings.motion);
  }
}

function saveSettings() {
  try { localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings)); } catch (e) { /* not fatal */ }
  applySettings();
}

$("#set-3d").checked = settings.scene3d;
$("#set-motion").checked = settings.motion;
$("#set-3d").addEventListener("change", (e) => { settings.scene3d = e.target.checked; saveSettings(); });
$("#set-motion").addEventListener("change", (e) => { settings.motion = e.target.checked; saveSettings(); });

$("#open-settings").addEventListener("click", () => {
  if (current !== "settings") backTo = current;
  $("#settings-note").hidden = true;
  show("settings");
});
$("#settings-back").addEventListener("click", () => show(backTo));
$("#open-data").addEventListener("click", () => invoke("open_folder", { kind: "data" }).catch(showError));
$("#open-logs").addEventListener("click", () => invoke("open_folder", { kind: "logs" }).catch(showError));

$("#repair").addEventListener("click", async () => {
  let st;
  try { st = await invoke("install_status"); } catch (e) { return showError(e); }
  const have = Object.keys(NAMES).filter((id) => st[id]);
  if (!have.length) {
    $("#settings-note").textContent = "Nothing is installed yet, so there's nothing to repair.";
    $("#settings-note").hidden = false;
    return;
  }
  installed = [];
  startInstall(have);
});

applySettings();
loadCommunity();
home();
