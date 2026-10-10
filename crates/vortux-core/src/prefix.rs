// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;
use std::process::Command;

use crate::error::{Error, Result};
use crate::paths::Paths;
use crate::progress::Progress;
use crate::system::find_in_path;

/// A `wine` command pointed at Vortux's own prefix.
pub fn wine_command(paths: &Paths) -> Command {
    let mut cmd = Command::new("wine");
    cmd.env("WINEPREFIX", paths.prefix_dir())
        .env("WINEDEBUG", "-all");
    cmd
}

/// Create the dedicated Wine prefix if it doesn't exist yet.
pub fn ensure_prefix(paths: &Paths, progress: &mut dyn Progress) -> Result<()> {
    let prefix = paths.prefix_dir();
    if prefix.join("system.reg").exists() {
        progress.step("Wine prefix already exists");
        return Ok(());
    }

    fs::create_dir_all(&prefix)?;
    progress.step("Creating the Wine prefix (this can take a minute)");

    let status = wine_command(paths)
        .args(["wineboot", "--init"])
        .env("WINEARCH", "win64")
        // Skip the Mono/Gecko download prompts; the Vortex apps don't need them.
        .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
        .status()?;
    if !status.success() {
        return Err(Error::Command(format!(
            "`wine wineboot --init` exited with {status}"
        )));
    }

    // The Vortex apps draw through DXVK and VKD3D-Proton, which a fresh prefix doesn't have.
    if find_in_path("winetricks").is_some() {
        progress.step("Installing DXVK and VKD3D-Proton into the prefix (winetricks)");
        let status = Command::new("winetricks")
            .args(["-q", "dxvk", "vkd3d"])
            .env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "-all")
            .status()?;
        if !status.success() {
            progress.warn("winetricks couldn't install DXVK/VKD3D-Proton; the apps may not run correctly");
        }
    } else {
        progress.warn("winetricks not found; skipped installing DXVK/VKD3D-Proton");
    }
    Ok(())
}
