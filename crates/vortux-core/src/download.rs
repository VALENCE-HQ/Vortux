// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};

const USER_AGENT: &str = concat!("vortux/", env!("CARGO_PKG_VERSION"), " (unofficial installer)");

/// Download `url` to `dest`. The data goes to `<dest>.part` first and is renamed when complete,
/// so a failed download never leaves a half-written file behind.
///
/// `on_progress` gets (bytes so far, total if known) and returns `false` to cancel.
pub fn download_to(
    url: &str,
    dest: &Path,
    on_progress: &mut dyn FnMut(u64, Option<u64>) -> bool,
) -> Result<()> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build();

    let response = match agent.get(url).set("User-Agent", USER_AGENT).call() {
        Ok(response) => response,
        // Studio's link answers 404 unless you're logged in to the website.
        Err(ureq::Error::Status(code, _)) if code == 403 || code == 404 => {
            return Err(Error::Refused(format!("{url}: HTTP {code}")));
        }
        Err(e) => return Err(Error::Download(format!("{url}: {e}"))),
    };
    let total = response.header("Content-Length").and_then(|v| v.parse::<u64>().ok());

    let mut part_name: OsString = dest.as_os_str().to_owned();
    part_name.push(".part");
    let part = PathBuf::from(part_name);

    let mut reader = response.into_reader();
    let mut file = File::create(&part)?;
    let mut buf = [0u8; 64 * 1024];
    let mut written: u64 = 0;

    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let _ = fs::remove_file(&part);
                return Err(Error::Download(format!("{url}: {e}")));
            }
        };
        file.write_all(&buf[..n])?;
        written += n as u64;
        if !on_progress(written, total) {
            drop(file);
            let _ = fs::remove_file(&part);
            return Err(Error::Cancelled);
        }
    }
    file.flush()?;
    drop(file);

    if let Some(expected) = total {
        if expected != written {
            let _ = fs::remove_file(&part);
            return Err(Error::Download(format!(
                "{url}: incomplete download ({written} of {expected} bytes)"
            )));
        }
    }

    fs::rename(&part, dest)?;
    Ok(())
}
