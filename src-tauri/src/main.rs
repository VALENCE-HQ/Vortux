// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State, WebviewWindow};
use vortux_core::{system, App, Error, InstallOptions, Paths, Progress, UninstallOptions};

/// Set by the Cancel button; the install polls it between chunks and steps.
struct CancelFlag(Arc<AtomicBool>);

#[derive(Serialize)]
struct SystemInfo {
    wine: Option<String>,
    winetricks: bool,
    xdg_mime: bool,
    vulkan: String,
    problems: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct InstallStatus {
    player: bool,
    studio: bool,
}

#[derive(Serialize, Clone)]
struct DownloadPayload {
    app: &'static str,
    name: &'static str,
    done: u64,
    total: Option<u64>,
}

#[derive(Serialize, Clone)]
struct DonePayload {
    ok: bool,
    cancelled: bool,
    error: Option<String>,
    /// Apps whose download the server refused; the window offers a manual file drop for these.
    skipped: Vec<&'static str>,
}

fn done(result: Result<Vec<App>, Error>) -> DonePayload {
    match result {
        Ok(skipped) => DonePayload {
            ok: true,
            cancelled: false,
            error: None,
            skipped: skipped.iter().map(|a| a.id()).collect(),
        },
        Err(Error::Cancelled) => DonePayload { ok: false, cancelled: true, error: None, skipped: vec![] },
        Err(e) => DonePayload { ok: false, cancelled: false, error: Some(e.to_string()), skipped: vec![] },
    }
}

/// Forwards core progress to the window as events.
struct TauriProgress {
    handle: AppHandle,
    cancel: Arc<AtomicBool>,
    last_pct: Option<u64>,
}

impl Progress for TauriProgress {
    fn step(&mut self, message: &str) {
        self.last_pct = None;
        let _ = self.handle.emit("vortux-step", message.to_string());
    }

    fn warn(&mut self, message: &str) {
        let _ = self.handle.emit("vortux-warn", message.to_string());
    }

    fn download(&mut self, app: App, done: u64, total: Option<u64>) {
        let pct = total.filter(|t| *t > 0).map(|t| done * 100 / t);
        if pct.is_some() && pct == self.last_pct {
            return;
        }
        self.last_pct = pct;
        let payload = DownloadPayload { app: app.id(), name: app.display_name(), done, total };
        let _ = self.handle.emit("vortux-download", payload);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

#[tauri::command]
fn system_check() -> SystemInfo {
    let r = system::check();
    SystemInfo {
        wine: r.wine_version.clone(),
        winetricks: r.winetricks,
        xdg_mime: r.xdg_mime,
        vulkan: format!("{:?}", r.vulkan),
        problems: r.blocking_problems(),
        warnings: r.warnings(),
    }
}

#[tauri::command]
fn install_status() -> Result<InstallStatus, String> {
    let paths = Paths::detect().map_err(|e| e.to_string())?;
    Ok(InstallStatus {
        player: paths.exe_path(App::Player).exists(),
        studio: paths.exe_path(App::Studio).exists(),
    })
}

fn spawn_install(handle: AppHandle, flag: &CancelFlag, opts: InstallOptions) -> Result<(), String> {
    let paths = Paths::detect().map_err(|e| e.to_string())?;
    flag.0.store(false, Ordering::Relaxed);
    let cancel = flag.0.clone();
    std::thread::spawn(move || {
        let mut progress = TauriProgress { handle: handle.clone(), cancel, last_pct: None };
        let result = vortux_core::install(&paths, &opts, &mut progress);
        let _ = handle.emit("vortux-done", done(result));
    });
    Ok(())
}

/// Runs the install on a background thread; progress and the result arrive as `vortux-*` events.
#[tauri::command]
fn start_install(handle: AppHandle, flag: State<'_, CancelFlag>, apps: Vec<String>) -> Result<(), String> {
    let apps: Vec<App> = apps.iter().filter_map(|id| App::from_id(id)).collect();
    if apps.is_empty() {
        return Err("Pick at least one app to install.".into());
    }
    spawn_install(handle, &flag, InstallOptions { apps, ..Default::default() })
}

/// Install one app from a zip/exe the user downloaded in their browser.
#[tauri::command]
fn install_file(handle: AppHandle, flag: State<'_, CancelFlag>, id: String, path: String) -> Result<(), String> {
    let app = App::from_id(&id).ok_or("unknown app")?;
    let opts = InstallOptions { apps: vec![app], local_files: vec![(app, PathBuf::from(path))] };
    spawn_install(handle, &flag, opts)
}

#[tauri::command]
fn cancel_install(flag: State<'_, CancelFlag>) {
    flag.0.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn start_uninstall(handle: AppHandle) -> Result<(), String> {
    let paths = Paths::detect().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut progress = TauriProgress { handle: handle.clone(), cancel, last_pct: None };
        let result = vortux_core::uninstall(&paths, &UninstallOptions::default(), &mut progress);
        let _ = handle.emit("vortux-done", done(result.map(|_| Vec::new())));
    });
    Ok(())
}

#[tauri::command]
fn launch_app(id: String) -> Result<(), String> {
    let app = App::from_id(&id).ok_or("unknown app")?;
    let paths = Paths::detect().map_err(|e| e.to_string())?;
    vortux_core::launch(&paths, app, None).map_err(|e| e.to_string())
}

/// Opens Vortux's data folder or its logs folder in the file manager.
#[tauri::command]
fn open_folder(kind: String) -> Result<(), String> {
    let paths = Paths::detect().map_err(|e| e.to_string())?;
    let dir = if kind == "logs" { paths.logs_dir() } else { paths.data_dir() };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("xdg-open")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("couldn't open {}: {e}", dir.display()))?;
    Ok(())
}

/// The install screen hides the system title bar, like a game launcher.
#[tauri::command]
fn set_frameless(window: WebviewWindow, on: bool) {
    let _ = window.set_decorations(!on);
}

/// The installed copy of this program is what menu entries and browser links run, as
/// `vortux launch <app> [link]`. In that mode no window is shown.
fn run_launch(args: &[String]) -> i32 {
    let Some(app) = args.first().and_then(|id| App::from_id(id)) else {
        eprintln!("usage: vortux launch <player|studio> [link]");
        return 2;
    };
    let url = args.get(1).map(String::as_str).filter(|u| !u.is_empty());
    let Ok(paths) = Paths::detect() else {
        return 1;
    };
    match vortux_core::launch(&paths, app, url) {
        Ok(()) => 0,
        Err(e) => {
            vortux_core::log_error(&paths, &format!("launch {}: {e}", app.id()));
            eprintln!("error: {e}");
            1
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("launch") {
        std::process::exit(run_launch(&args[1..]));
    }

    tauri::Builder::default()
        .manage(CancelFlag(Arc::new(AtomicBool::new(false))))
        .invoke_handler(tauri::generate_handler![
            system_check,
            install_status,
            start_install,
            install_file,
            cancel_install,
            start_uninstall,
            launch_app,
            set_frameless,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vortux");
}
