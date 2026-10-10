// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths::Paths;

/// Append a line to `vortux.log`. Launches started from the browser have no terminal,
/// so this is where their errors end up. Best effort: failures to log are ignored.
pub fn log_error(paths: &Paths, message: &str) {
    let _ = fs::create_dir_all(paths.data_dir());
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.data_dir().join("vortux.log"))
    {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {message}");
    }
}
