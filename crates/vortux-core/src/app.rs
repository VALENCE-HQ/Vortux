// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// The two Vortex apps Vortux manages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    Player,
    Studio,
}

impl App {
    pub const ALL: [App; 2] = [App::Player, App::Studio];

    /// Short id used in file names and on the command line.
    pub fn id(self) -> &'static str {
        match self {
            App::Player => "player",
            App::Studio => "studio",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            App::Player => "Vortux Player",
            App::Studio => "Vortux Studio",
        }
    }

    pub fn comment(self) -> &'static str {
        match self {
            App::Player => "Play Vortex on Linux (unofficial, runs through Wine)",
            App::Studio => "Vortex Studio on Linux (unofficial, runs through Wine)",
        }
    }

    /// File name the exe is saved under.
    pub fn exe_name(self) -> &'static str {
        match self {
            App::Player => "Vortex.exe",
            App::Studio => "Vortex-Studio.exe",
        }
    }

    /// Names to prefer when several .exe files turn up after unpacking.
    pub fn exe_candidates(self) -> &'static [&'static str] {
        match self {
            App::Player => &["Vortex.exe", "Vortex-Launcher.exe"],
            App::Studio => &["Vortex-Studio.exe", "VortexStudio.exe", "Studio.exe"],
        }
    }

    /// Official download link (served directly by playvortex.io).
    pub fn download_url(self) -> &'static str {
        match self {
            App::Player => "https://playvortex.io/download/windows",
            App::Studio => "https://playvortex.io/download/studio-windows",
        }
    }

    /// URL scheme the website uses to open this app (without "://").
    pub fn url_scheme(self) -> &'static str {
        match self {
            App::Player => "vortex",
            App::Studio => "vortex-studio",
        }
    }

    pub fn from_id(id: &str) -> Option<App> {
        match id.to_ascii_lowercase().as_str() {
            "player" => Some(App::Player),
            "studio" => Some(App::Studio),
            _ => None,
        }
    }

    /// Which app a link such as `vortex://play?...` belongs to.
    pub fn from_url(url: &str) -> Option<App> {
        let (scheme, _) = url.split_once("://")?;
        let scheme = scheme.to_ascii_lowercase();
        App::ALL.into_iter().find(|a| a.url_scheme() == scheme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_urls_to_the_right_app() {
        assert_eq!(
            App::from_url("vortex://play?game=15&token=x"),
            Some(App::Player)
        );
        assert_eq!(
            App::from_url("vortex-studio://open?token=x"),
            Some(App::Studio)
        );
        assert_eq!(App::from_url("https://playvortex.io"), None);
        assert_eq!(App::from_url("vortex"), None);
    }

    #[test]
    fn parses_ids() {
        assert_eq!(App::from_id("Player"), Some(App::Player));
        assert_eq!(App::from_id("studio"), Some(App::Studio));
        assert_eq!(App::from_id("nope"), None);
    }
}
