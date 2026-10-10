// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Core logic for Vortux, an unofficial Linux bootstrapper for Vortex and Vortex Studio.
//! Everything user-visible reports through [`Progress`], so any front end can drive it.

pub mod app;
pub mod archive;
pub mod download;
pub mod error;
pub mod icons;
pub mod install;
pub mod integrate;
pub mod launch;
pub mod log;
pub mod paths;
pub mod prefix;
pub mod progress;
pub mod system;
pub mod uninstall;
pub mod wine;

pub use app::App;
pub use error::{Error, Result};
pub use install::{install, InstallOptions};
pub use launch::launch;
pub use log::log_error;
pub use paths::Paths;
pub use progress::{NullProgress, Progress};
pub use uninstall::{uninstall, UninstallOptions};
pub use wine::{WineRelease, WineSource};
