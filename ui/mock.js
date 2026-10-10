// Design the UI without compiling Rust: run `python3 -m http.server -d ui 8000` and open
// http://localhost:8000/ in a browser. Options: ?installed  ?fail=studio  (simulates a refused download)
if (!window.__TAURI__) {
  const handlers = {};
  const emit = (n, p) => (handlers[n] || []).forEach((f) => f({ payload: p }));
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const q = new URLSearchParams(location.search);
  let cancel = false;

  async function fakeInstall(ids, manual) {
    cancel = false;
    for (const s of ["Checking your system", "Installing Vortux", "Creating the Wine prefix (this can take a minute)"]) {
      emit("vortux-step", s);
      await sleep(900);
    }
    const skipped = [];
    for (const id of ids) {
      const name = id === "player" ? "Vortux Player" : "Vortux Studio";
      emit("vortux-step", (manual ? "Copying " : "Downloading ") + name);
      if (!manual && q.get("fail") === id) { skipped.push(id); await sleep(500); continue; }
      for (let p = 0; p <= 100; p += 4) {
        if (cancel) return emit("vortux-done", { ok: false, cancelled: true, skipped: [] });
        emit("vortux-download", { app: id, name, done: p * 650000, total: 65000000 });
        await sleep(60);
      }
      emit("vortux-step", "Unpacking " + name); await sleep(700);
      emit("vortux-step", "Adding " + name + " to the app menu"); await sleep(500);
    }
    emit("vortux-step", "All done");
    await sleep(500);
    emit("vortux-done", { ok: true, cancelled: false, skipped, error: null });
  }

  window.__TAURI__ = {
    core: {
      invoke: async (cmd, a = {}) => {
        if (cmd === "system_check") return { wine: "wine-11.0 (Staging)", winetricks: true, xdg_mime: true, vulkan: "Working", problems: [], warnings: [] };
        if (cmd === "install_status") return { player: q.has("installed"), studio: false };
        if (cmd === "start_install") fakeInstall(a.apps, false);
        if (cmd === "install_file") fakeInstall([a.id], true);
        if (cmd === "cancel_install") cancel = true;
        if (cmd === "start_uninstall") sleep(1200).then(() => emit("vortux-done", { ok: true, cancelled: false, skipped: [], error: null }));
      },
    },
    event: { listen: async (n, f) => { (handlers[n] ||= []).push(f); return () => {}; } },
  };
}
