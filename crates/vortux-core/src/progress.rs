// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Everything long-running reports through this, so the terminal tool and the installer window
/// can both show progress without the core knowing about either.
pub trait Progress {
    fn step(&mut self, message: &str);
    fn warn(&mut self, message: &str);
    /// A file is downloading: `name` says what, `total` is None when the size isn't known.
    fn download(&mut self, name: &str, done: u64, total: Option<u64>);
    /// Long steps poll this so a Cancel button can stop them.
    fn is_cancelled(&self) -> bool {
        false
    }
}

pub struct NullProgress;

impl Progress for NullProgress {
    fn step(&mut self, _message: &str) {}
    fn warn(&mut self, _message: &str) {}
    fn download(&mut self, _name: &str, _done: u64, _total: Option<u64>) {}
}
