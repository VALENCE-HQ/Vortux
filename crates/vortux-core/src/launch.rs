// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::Stdio;

use crate::app::App;
use crate::error::{Error, Result};
use crate::paths::Paths;
use crate::prefix::wine_command;

/// Start an app through Wine, optionally handing it a login/play link.
/// Output goes to `logs/<id>.log` so problems can be diagnosed later.
pub fn launch(paths: &Paths, app: App, url: Option<&str>) -> Result<()> {
    let exe = paths.exe_path(app);
    if !exe.exists() {
        return Err(Error::Other(format!(
            "{} isn't installed yet (missing {}). Run `vortux install` first.",
            app.display_name(),
            exe.display()
        )));
    }

    // Links arrive from web pages, so only pass one through if it's really this app's link.
    if let Some(u) = url {
        if App::from_url(u) != Some(app) {
            return Err(Error::Other(format!(
                "refusing to pass an unexpected link to {}",
                app.display_name()
            )));
        }
    }

    fs::create_dir_all(paths.logs_dir())?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.log_file(app))?;
    let log_err = log.try_clone()?;

    // Run from the exe's own folder, since apps often look for files next to themselves.
    let workdir = exe
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths.app_dir(app));

    let mut cmd = wine_command(paths);
    cmd.current_dir(workdir)
        .arg(&exe)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    if let Some(u) = url {
        cmd.arg(u);
    }
    cmd.spawn()?;
    Ok(())
}
