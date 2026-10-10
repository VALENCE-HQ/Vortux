// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Vortux can bring its own Wine instead of relying on the one your distro ships. Builds come from
//! Kron4ek's Wine-Builds on GitHub; winetricks comes from its own repository.

use std::fs::{self, File};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::download;
use crate::error::{Error, Result};
use crate::paths::Paths;
use crate::progress::Progress;
use crate::system::find_in_path;

const RELEASES: &str = "https://api.github.com/repos/Kron4ek/Wine-Builds/releases";
/// The version we've tested with. If it isn't available we use the newest release instead.
const PREFERRED_TAG: &str = "11.0";
/// Build flavours in order of preference. The wow64 build needs no 32-bit libraries.
const SUFFIXES: [&str; 2] = ["-staging-amd64-wow64.tar.xz", "-staging-amd64.tar.xz"];
const WINETRICKS_URLS: [&str; 2] = [
    "https://raw.githubusercontent.com/Winetricks/winetricks/20260125/src/winetricks",
    "https://raw.githubusercontent.com/Winetricks/winetricks/master/src/winetricks",
];
const USER_AGENT: &str = concat!(
    "vortux/",
    env!("CARGO_PKG_VERSION"),
    " (unofficial installer)"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WineSource {
    /// Vortux's own copy, kept in its data folder.
    Bundled,
    /// Whatever `wine` the system provides.
    System,
}

#[derive(Debug, Clone)]
pub struct WineRelease {
    /// For example `wine-11.0-staging-amd64-wow64`.
    pub name: String,
    pub size: u64,
    pub url: String,
    pub sha256: Option<String>,
}

pub fn bundled_binary(paths: &Paths) -> Option<PathBuf> {
    let bin = paths.wine_dir().join("bin/wine");
    bin.exists().then_some(bin)
}

/// The wine to run: Vortux's own if it's installed, otherwise the system one.
pub fn binary(paths: &Paths) -> PathBuf {
    bundled_binary(paths).unwrap_or_else(|| PathBuf::from("wine"))
}

/// What to use when nobody chose: our own Wine if we have it, else the system's, else download one.
pub fn auto(paths: &Paths) -> WineSource {
    if bundled_binary(paths).is_some() {
        WineSource::Bundled
    } else if find_in_path("wine").is_some() {
        WineSource::System
    } else {
        WineSource::Bundled
    }
}

/// Point a command (winetricks) at Vortux's own Wine, if we have one.
pub fn use_bundled(paths: &Paths, cmd: &mut Command) {
    let Some(bin) = bundled_binary(paths) else {
        return;
    };
    let Some(dir) = bin.parent() else {
        return;
    };
    cmd.env("WINE", &bin)
        .env("WINESERVER", dir.join("wineserver"));
    let old = std::env::var_os("PATH").unwrap_or_default();
    let search = std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&old));
    if let Ok(joined) = std::env::join_paths(search) {
        cmd.env("PATH", joined);
    }
}

fn fetch_release(url: &str) -> Result<Option<Value>> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build();
    let request = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json");
    match request.call() {
        Ok(response) => {
            let text = response.into_string()?;
            serde_json::from_str(&text)
                .map(Some)
                .map_err(|e| Error::Other(format!("unexpected reply from GitHub: {e}")))
        }
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(e) => Err(Error::Download(format!("{url}: {e}"))),
    }
}

/// Looks up which Wine build to download (needs the internet, but downloads nothing).
pub fn latest_release() -> Result<WineRelease> {
    let release = match fetch_release(&format!("{RELEASES}/tags/{PREFERRED_TAG}"))? {
        Some(release) => release,
        None => fetch_release(&format!("{RELEASES}/latest"))?
            .ok_or_else(|| Error::Other("couldn't find a Wine release to download".into()))?,
    };
    pick_asset(&release)
}

fn pick_asset(release: &Value) -> Result<WineRelease> {
    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| Error::Other("the Wine release has no files".into()))?;
    for suffix in SUFFIXES {
        for asset in assets {
            let file = asset["name"].as_str().unwrap_or("");
            if !(file.starts_with("wine-") && file.ends_with(suffix)) {
                continue;
            }
            let url = asset["browser_download_url"]
                .as_str()
                .ok_or_else(|| Error::Other("the Wine release has no download link".into()))?;
            return Ok(WineRelease {
                name: file.trim_end_matches(".tar.xz").to_string(),
                size: asset["size"].as_u64().unwrap_or(0),
                url: url.to_string(),
                sha256: checksum(release, asset, file),
            });
        }
    }
    Err(Error::Other(
        "no matching Wine build in that release".into(),
    ))
}

/// The release notes list checksums as `<hash>  <file name>`; newer GitHub releases also expose a digest.
fn checksum(release: &Value, asset: &Value, file: &str) -> Option<String> {
    if let Some(digest) = asset["digest"]
        .as_str()
        .and_then(|d| d.strip_prefix("sha256:"))
    {
        return Some(digest.to_lowercase());
    }
    release["body"].as_str()?.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        let (hash, name) = (words.next()?, words.next()?);
        (name == file && words.next().is_none()).then(|| hash.to_lowercase())
    })
}

fn sha256_of(path: &std::path::Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Download Wine and unpack it into Vortux's data folder.
pub fn install_bundled(
    paths: &Paths,
    release: &WineRelease,
    progress: &mut dyn Progress,
) -> Result<()> {
    if bundled_binary(paths).is_some() {
        return Ok(());
    }
    fs::create_dir_all(paths.data_dir())?;
    let archive = paths.data_dir().join("wine.tar.xz");

    progress.step(&format!("Downloading {}", release.name));
    download::download_to(&release.url, &archive, &mut |done, total| {
        progress.download(&release.name, done, total);
        !progress.is_cancelled()
    })?;

    if let Some(expected) = &release.sha256 {
        progress.step("Checking the download");
        if sha256_of(&archive)? != *expected {
            let _ = fs::remove_file(&archive);
            return Err(Error::Download(
                "the Wine download is corrupted (checksum mismatch)".into(),
            ));
        }
    }

    progress.step("Unpacking Wine");
    let staging = paths.data_dir().join("wine.new");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;
    let status = Command::new("tar")
        .arg("-xJf")
        .arg(&archive)
        .arg("-C")
        .arg(&staging)
        .arg("--strip-components=1")
        .status()?;
    let _ = fs::remove_file(&archive);
    if !status.success() {
        let _ = fs::remove_dir_all(&staging);
        return Err(Error::Command(format!(
            "`tar` exited with {status} while unpacking Wine"
        )));
    }
    let _ = fs::remove_dir_all(paths.wine_dir());
    fs::rename(&staging, paths.wine_dir())?;
    Ok(())
}

/// Vortux keeps its own copy of winetricks so it doesn't depend on a distro package.
pub fn ensure_winetricks(paths: &Paths, progress: &mut dyn Progress) -> Result<PathBuf> {
    let dest = paths.winetricks_path();
    if dest.exists() {
        return Ok(dest);
    }
    progress.step("Downloading winetricks");
    fs::create_dir_all(paths.tools_dir())?;
    let mut refused = None;
    for url in WINETRICKS_URLS {
        match download::download_to(url, &dest, &mut |_, _| true) {
            Ok(()) => {
                fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
                return Ok(dest);
            }
            Err(Error::Refused(why)) => refused = Some(why),
            Err(e) => return Err(e),
        }
    }
    Err(Error::Download(
        refused.unwrap_or_else(|| "winetricks".into()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefers_the_wow64_staging_build_and_finds_its_checksum() {
        let release = json!({
            "body": "SHA256\nabc123  wine-11.0-staging-amd64-wow64.tar.xz\nff00  wine-11.0-staging-amd64.tar.xz\n",
            "assets": [
                { "name": "wine-11.0-staging-tkg-amd64.tar.xz", "size": 1, "browser_download_url": "https://x/tkg" },
                { "name": "wine-11.0-staging-amd64.tar.xz", "size": 2, "browser_download_url": "https://x/plain" },
                { "name": "wine-11.0-staging-amd64-wow64.tar.xz", "size": 3, "browser_download_url": "https://x/wow64" }
            ]
        });
        let pick = pick_asset(&release).unwrap();
        assert_eq!(pick.url, "https://x/wow64");
        assert_eq!(pick.name, "wine-11.0-staging-amd64-wow64");
        assert_eq!(pick.sha256.as_deref(), Some("abc123"));
    }

    #[test]
    fn falls_back_to_the_plain_staging_build() {
        let release = json!({ "assets": [
            { "name": "wine-proton-11.0-1-amd64.tar.xz", "size": 1, "browser_download_url": "https://x/proton" },
            { "name": "wine-11.0-staging-amd64.tar.xz", "size": 2, "browser_download_url": "https://x/plain" }
        ] });
        assert_eq!(pick_asset(&release).unwrap().url, "https://x/plain");
    }
}
