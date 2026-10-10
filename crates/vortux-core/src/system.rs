// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::wine::WineSource;

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
    /// Version of the Wine installed on the system, if any.
    pub wine_version: Option<String>,
    pub xdg_mime: bool,
    pub update_desktop_database: bool,
    pub tar: bool,
    pub xz: bool,
    pub zstd: bool,
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
        Some(tool) => {
            let ok = Command::new(tool)
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
        xdg_mime: find_in_path("xdg-mime").is_some(),
        update_desktop_database: find_in_path("update-desktop-database").is_some(),
        tar: find_in_path("tar").is_some(),
        xz: find_in_path("xz").is_some(),
        zstd: find_in_path("zstd").is_some(),
        vulkan,
    }
}

impl SystemReport {
    /// Problems that stop the install. `bundled_ready` is true when Vortux's own Wine is
    /// already unpacked, so `tar` and `xz` aren't needed.
    pub fn blocking_problems(&self, wine: WineSource, bundled_ready: bool) -> Vec<String> {
        let mut v = Vec::new();
        match wine {
            WineSource::System if self.wine_version.is_none() => {
                v.push("Wine isn't installed on your system. Pick \"Download Wine for Vortux\" or install your distro's wine package.".to_string());
            }
            WineSource::Bundled if !bundled_ready => {
                if !self.tar {
                    v.push("`tar` wasn't found. It's needed to unpack Wine.".to_string());
                }
                if !self.xz {
                    v.push("`xz` wasn't found. Install your distro's xz (or xz-utils) package to unpack Wine.".to_string());
                }
            }
            _ => {}
        }
        if !self.zstd {
            v.push("`zstd` wasn't found. It's needed to unpack the graphics components (VKD3D-Proton).".to_string());
        }
        if !self.xdg_mime {
            v.push("`xdg-mime` wasn't found. Install your distro's xdg-utils package.".to_string());
        }
        if self.vulkan == Vulkan::NotWorking {
            v.push("Vulkan doesn't seem to be working (`vulkaninfo` failed). Install your GPU's Vulkan driver (Mesa RADV for AMD).".to_string());
        }
        v
    }

    /// Things worth mentioning that don't stop the install.
    pub fn warnings(&self) -> Vec<String> {
        let mut v = Vec::new();
        if !self.update_desktop_database {
            v.push("`update-desktop-database` wasn't found; the app menu entries may take a while to appear.".to_string());
        }
        if self.vulkan == Vulkan::Unknown {
            v.push("`vulkaninfo` wasn't found, so Vulkan couldn't be checked.".to_string());
        }
        v
    }
}
