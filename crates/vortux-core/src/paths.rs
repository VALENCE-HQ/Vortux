// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::fs;
use std::path::PathBuf;

use crate::app::App;
use crate::error::{Error, Result};

/// File inside each app folder that records where its exe ended up after unpacking.
pub const EXE_MARKER: &str = ".vortux-exe";
/// Temporary name for a download before it's unpacked.
pub const DOWNLOAD_FILE: &str = ".download";

/// Where Vortux keeps everything. Layout under `$XDG_DATA_HOME` (default `~/.local/share`):
///
/// ```text
/// vortux/bin/vortux          the installed copy of Vortux itself
/// vortux/prefix/             dedicated Wine prefix
/// vortux/apps/<id>/          unpacked Vortex apps
/// vortux/logs/<id>.log       output of launched apps
/// applications/vortux-<id>.desktop
/// ```
#[derive(Debug, Clone)]
pub struct Paths {
    data_home: PathBuf,
}

impl Paths {
    pub fn detect() -> Result<Self> {
        let data_home = match env::var_os("XDG_DATA_HOME") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => {
                let home = env::var_os("HOME")
                    .ok_or_else(|| Error::Other("HOME is not set".into()))?;
                PathBuf::from(home).join(".local/share")
            }
        };
        Ok(Self { data_home })
    }

    pub fn with_data_home(data_home: PathBuf) -> Self {
        Self { data_home }
    }

    pub fn data_dir(&self) -> PathBuf {
        self.data_home.join("vortux")
    }
    pub fn bin_dir(&self) -> PathBuf {
        self.data_dir().join("bin")
    }
    pub fn vortux_bin(&self) -> PathBuf {
        self.bin_dir().join("vortux")
    }
    pub fn prefix_dir(&self) -> PathBuf {
        self.data_dir().join("prefix")
    }
    pub fn apps_dir(&self) -> PathBuf {
        self.data_dir().join("apps")
    }
    pub fn app_dir(&self, app: App) -> PathBuf {
        self.apps_dir().join(app.id())
    }
    /// The app's exe. The downloads are zip archives, so the real location is recorded in
    /// `EXE_MARKER` at install time; before that we fall back to the expected name.
    pub fn exe_path(&self, app: App) -> PathBuf {
        let dir = self.app_dir(app);
        match fs::read_to_string(dir.join(EXE_MARKER)) {
            Ok(rel) if !rel.trim().is_empty() => dir.join(rel.trim()),
            _ => dir.join(app.exe_name()),
        }
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir().join("logs")
    }
    pub fn log_file(&self, app: App) -> PathBuf {
        self.logs_dir().join(format!("{}.log", app.id()))
    }
    pub fn icon_file(&self, app: App, size: u32) -> PathBuf {
        self.data_home
            .join(format!("icons/hicolor/{size}x{size}/apps/vortux-{}.png", app.id()))
    }
    pub fn desktop_dir(&self) -> PathBuf {
        self.data_home.join("applications")
    }
    pub fn desktop_file(&self, app: App) -> PathBuf {
        self.desktop_dir().join(format!("vortux-{}.desktop", app.id()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_stable() {
        let p = Paths::with_data_home(PathBuf::from("/tmp/x"));
        assert_eq!(
            p.exe_path(App::Player),
            PathBuf::from("/tmp/x/vortux/apps/player/Vortex.exe")
        );
        assert_eq!(
            p.desktop_file(App::Studio),
            PathBuf::from("/tmp/x/applications/vortux-studio.desktop")
        );
        assert_eq!(p.vortux_bin(), PathBuf::from("/tmp/x/vortux/bin/vortux"));
    }
}
