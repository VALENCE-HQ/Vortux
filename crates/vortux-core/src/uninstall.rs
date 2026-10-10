// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::app::App;
use crate::error::Result;
use crate::icons;
use crate::integrate;
use crate::paths::Paths;
use crate::progress::Progress;

pub struct UninstallOptions {
    /// Delete the downloaded Vortex apps.
    pub remove_apps: bool,
    /// Delete the Wine prefix (this also removes saved Vortex settings inside it).
    pub remove_prefix: bool,
}

impl Default for UninstallOptions {
    fn default() -> Self {
        Self {
            remove_apps: true,
            remove_prefix: true,
        }
    }
}

pub fn uninstall(
    paths: &Paths,
    opts: &UninstallOptions,
    progress: &mut dyn Progress,
) -> Result<()> {
    progress.step("Removing app menu entries");
    for app in App::ALL {
        remove_file(&paths.desktop_file(app))?;
        icons::remove(paths, app);
    }
    integrate::refresh_database(paths);

    if opts.remove_apps {
        progress.step("Removing downloaded apps");
        remove_dir(&paths.apps_dir())?;
    }
    if opts.remove_prefix {
        progress.step("Removing the Wine prefix");
        remove_dir(&paths.prefix_dir())?;
    }
    if opts.remove_apps && opts.remove_prefix {
        progress.step("Removing Vortux itself");
        remove_dir(&paths.bin_dir())?;
        remove_dir(&paths.logs_dir())?;
        remove_dir(&paths.wine_dir())?;
        remove_dir(&paths.tools_dir())?;
        // Only succeeds if empty; that's what we want.
        let _ = fs::remove_dir(paths.data_dir());
    }
    progress.step("Uninstall finished");
    Ok(())
}

fn remove_file(p: &Path) -> Result<()> {
    match fs::remove_file(p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn remove_dir(p: &Path) -> Result<()> {
    match fs::remove_dir_all(p) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
