// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::app::App;
use crate::archive;
use crate::download;
use crate::error::{Error, Result};
use crate::icons;
use crate::integrate;
use crate::paths::{Paths, DOWNLOAD_FILE, EXE_MARKER};
use crate::prefix;
use crate::progress::Progress;
use crate::system;

pub struct InstallOptions {
    pub apps: Vec<App>,
    /// Apps to install from a zip/exe the user downloaded themselves instead of downloading.
    pub local_files: Vec<(App, PathBuf)>,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self { apps: App::ALL.to_vec(), local_files: Vec::new() }
    }
}

/// Installs the chosen apps. Returns the apps that were skipped because the server refused the
/// download (they can be installed later from a file the user downloads in a browser).
pub fn install(paths: &Paths, opts: &InstallOptions, progress: &mut dyn Progress) -> Result<Vec<App>> {
    progress.step("Checking your system");
    let report = system::check();
    let problems = report.blocking_problems();
    if !problems.is_empty() {
        return Err(Error::Preflight(problems));
    }
    for w in report.warnings() {
        progress.warn(&w);
    }

    fs::create_dir_all(paths.bin_dir())?;
    fs::create_dir_all(paths.desktop_dir())?;

    progress.step("Installing Vortux");
    install_self(paths)?;

    if progress.is_cancelled() {
        return Err(Error::Cancelled);
    }
    prefix::ensure_prefix(paths, progress)?;

    let mut skipped = Vec::new();
    for &app in &opts.apps {
        if progress.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let local = opts.local_files.iter().find(|(a, _)| *a == app).map(|(_, p)| p.as_path());
        match install_app(paths, app, local, progress) {
            Ok(()) => {}
            Err(Error::Refused(why)) => {
                progress.warn(&format!("{} can't be downloaded automatically ({why})", app.display_name()));
                skipped.push(app);
                continue;
            }
            Err(e) => return Err(e),
        }

        progress.step(&format!("Adding {} to the app menu", app.display_name()));
        integrate::register(paths, app)?;
        icons::install(paths, app)?;
        if let Err(e) = integrate::register_wine_protocol(paths, app) {
            progress.warn(&format!(
                "couldn't add the Wine registry entry for {}: {e}",
                app.display_name()
            ));
        }
    }

    integrate::refresh_database(paths);
    progress.step("All done");
    Ok(skipped)
}

/// Get one app's file (downloaded, or copied from `local`) and put its exe in place. The official
/// downloads are zip archives (e.g. `Vortex/Vortex.exe`), but a bare exe is accepted too.
fn install_app(paths: &Paths, app: App, local: Option<&Path>, progress: &mut dyn Progress) -> Result<()> {
    let app_dir = paths.app_dir(app);
    fs::create_dir_all(&app_dir)?;
    let download_path = app_dir.join(DOWNLOAD_FILE);

    match local {
        Some(src) => {
            progress.step(&format!("Copying {}", app.display_name()));
            fs::copy(src, &download_path)?;
        }
        None => {
            progress.step(&format!("Downloading {}", app.display_name()));
            download::download_to(app.download_url(), &download_path, &mut |done, total| {
                progress.download(app, done, total);
                !progress.is_cancelled()
            })?;
        }
    }

    if archive::looks_like_exe(&download_path)? {
        fs::rename(&download_path, app_dir.join(app.exe_name()))?;
    } else {
        progress.step(&format!("Unpacking {}", app.display_name()));
        let result = archive::extract_zip(&download_path, &app_dir);
        let _ = fs::remove_file(&download_path);
        result?;
    }

    let exe = archive::find_exe(&app_dir, app.exe_candidates())?;
    fs::write(app_dir.join(EXE_MARKER), exe.to_string_lossy().as_bytes())?;
    Ok(())
}

/// Copy this program to a stable location so menu entries and link handlers don't break
/// when the downloaded file is moved or deleted. For an AppImage, copy the .AppImage itself.
fn install_self(paths: &Paths) -> Result<()> {
    let src: PathBuf = match env::var_os("APPIMAGE") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => env::current_exe()?,
    };
    let dest = paths.vortux_bin();

    let same = match (fs::canonicalize(&src), fs::canonicalize(&dest)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same {
        return Ok(());
    }

    // Copy next to the target and rename over it, since a running binary can't be overwritten in place.
    let tmp = dest.with_extension("new");
    fs::copy(&src, &tmp)?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    fs::rename(&tmp, &dest)?;
    Ok(())
}
