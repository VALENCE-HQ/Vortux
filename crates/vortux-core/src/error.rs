// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Download(String),
    /// System check failed; each entry is a human-readable problem.
    Preflight(Vec<String>),
    Command(String),
    Other(String),
    Cancelled,
    /// The server said no (403/404), e.g. a download that needs a logged-in account.
    Refused(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Download(m) => write!(f, "download failed: {m}"),
            Error::Preflight(list) => {
                write!(f, "this system isn't ready:")?;
                for item in list {
                    write!(f, "\n  - {item}")?;
                }
                Ok(())
            }
            Error::Command(m) => write!(f, "command failed: {m}"),
            Error::Other(m) => write!(f, "{m}"),
            Error::Cancelled => write!(f, "cancelled"),
            Error::Refused(m) => write!(f, "the server refused the download: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
