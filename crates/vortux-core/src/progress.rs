// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::app::App;

/// Everything long-running reports through this, so the terminal tool now and the
/// Tauri installer window later can both show progress without changing the core.
pub trait Progress {
    fn step(&mut self, message: &str);
    fn warn(&mut self, message: &str);
    fn download(&mut self, app: App, done: u64, total: Option<u64>);
    /// Long steps poll this so a Cancel button can stop them.
    fn is_cancelled(&self) -> bool {
        false
    }
}

pub struct NullProgress;

impl Progress for NullProgress {
    fn step(&mut self, _message: &str) {}
    fn warn(&mut self, _message: &str) {}
    fn download(&mut self, _app: App, _done: u64, _total: Option<u64>) {}
}
