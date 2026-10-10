// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| is_executable(p))
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vulkan {
    Working,
    NotWorking,
    /// `vulkaninfo` isn't installed, so we can't tell.
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SystemReport {
    pub wine_version: Option<String>,
    pub winetricks: bool,
    pub xdg_mime: bool,
    pub update_desktop_database: bool,
    pub vulkan: Vulkan,
}

pub fn check() -> SystemReport {
    let wine_version = find_in_path("wine").and_then(|_| {
        let out = Command::new("wine").arg("--version").output().ok()?;
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            None
        }
    });

    let vulkan = match find_in_path("vulkaninfo") {
        None => Vulkan::Unknown,
        Some(p) => {
            let ok = Command::new(p)
                .arg("--summary")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                Vulkan::Working
            } else {
                Vulkan::NotWorking
            }
        }
    };

    SystemReport {
        wine_version,
        winetricks: find_in_path("winetricks").is_some(),
        xdg_mime: find_in_path("xdg-mime").is_some(),
        update_desktop_database: find_in_path("update-desktop-database").is_some(),
        vulkan,
    }
}

impl SystemReport {
    /// Problems that stop the install.
    pub fn blocking_problems(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.wine_version.is_none() {
            v.push("Wine isn't installed (the `wine` command wasn't found). Install your distro's wine package.".to_string());
        }
        if !self.xdg_mime {
            v.push("`xdg-mime` wasn't found. Install your distro's xdg-utils package.".to_string());
        }
        if self.vulkan == Vulkan::NotWorking {
            v.push("Vulkan doesn't seem to be working (`vulkaninfo` failed). Install your GPU's Vulkan driver (Mesa RADV for AMD).".to_string());
        }
        v
    }

    /// Things worth telling the user about that don't stop the install.
    pub fn warnings(&self) -> Vec<String> {
        let mut v = Vec::new();
        if !self.winetricks {
            v.push("`winetricks` wasn't found, so DXVK and VKD3D-Proton can't be installed into the Wine prefix automatically. The apps may run slowly or not at all without them.".to_string());
        }
        if !self.update_desktop_database {
            v.push("`update-desktop-database` wasn't found; the app menu entries may take a while to appear.".to_string());
        }
        if self.vulkan == Vulkan::Unknown {
            v.push("`vulkaninfo` wasn't found, so Vulkan couldn't be checked.".to_string());
        }
        v
    }
}
