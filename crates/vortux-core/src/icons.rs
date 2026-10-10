// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;

use crate::app::App;
use crate::error::Result;
use crate::paths::Paths;

const SIZES: [u32; 3] = [48, 128, 256];

const PLAYER_48: &[u8] = include_bytes!("../assets/icon-48.png");
const PLAYER_128: &[u8] = include_bytes!("../assets/icon-128.png");
const PLAYER_256: &[u8] = include_bytes!("../assets/icon-256.png");
const STUDIO_48: &[u8] = include_bytes!("../assets/studio-icon-48.png");
const STUDIO_128: &[u8] = include_bytes!("../assets/studio-icon-128.png");
const STUDIO_256: &[u8] = include_bytes!("../assets/studio-icon-256.png");

fn images(app: App) -> [&'static [u8]; 3] {
    match app {
        App::Player => [PLAYER_48, PLAYER_128, PLAYER_256],
        App::Studio => [STUDIO_48, STUDIO_128, STUDIO_256],
    }
}

/// Save the app's icon. The menu entry points at the 256px file by absolute path, so the
/// desktop doesn't need to refresh its icon cache for it to show up.
pub fn install(paths: &Paths, app: App) -> Result<()> {
    for (size, bytes) in SIZES.into_iter().zip(images(app)) {
        let file = paths.icon_file(app, size);
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&file, bytes)?;
    }
    Ok(())
}

pub fn remove(paths: &Paths, app: App) {
    for size in SIZES {
        let _ = fs::remove_file(paths.icon_file(app, size));
    }
}
