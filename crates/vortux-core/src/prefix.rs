// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;
use std::process::Command;

use crate::error::{Error, Result};
use crate::paths::Paths;
use crate::progress::Progress;
use crate::wine;

/// A `wine` command pointed at Vortux's own prefix.
pub fn wine_command(paths: &Paths) -> Command {
    let mut cmd = Command::new(wine::binary(paths));
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
    let winetricks = wine::ensure_winetricks(paths, progress)?;
    progress.step("Installing DXVK and VKD3D-Proton (about 22 MB)");
    let mut cmd = Command::new(winetricks);
    cmd.args(["-q", "dxvk", "vkd3d"])
        .env("WINEPREFIX", &prefix)
        .env("WINEDEBUG", "-all");
    wine::use_bundled(paths, &mut cmd);
    if !cmd.status()?.success() {
        progress
            .warn("winetricks couldn't install DXVK/VKD3D-Proton; the apps may not run correctly");
    }
    Ok(())
}
