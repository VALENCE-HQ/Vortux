// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::app::App;
use crate::error::{Error, Result};
use crate::paths::Paths;
use crate::prefix::wine_command;
use crate::system::find_in_path;

/// Quote one argument for the `Exec=` line of a .desktop file.
/// Inside double quotes, `"`, backtick, `$` and `\` get a backslash, `%` is doubled, and
/// (because desktop-file strings have their own escaping layer) every backslash is doubled again.
pub fn quote_exec_arg(s: &str) -> String {
    let mut level1 = String::new();
    for ch in s.chars() {
        match ch {
            '"' | '`' | '$' | '\\' => {
                level1.push('\\');
                level1.push(ch);
            }
            '%' => level1.push_str("%%"),
            _ => level1.push(ch),
        }
    }
    format!("\"{}\"", level1.replace('\\', "\\\\"))
}

/// The app-menu entry. It also claims the app's URL scheme, so the website's play button
/// opens it. `%u` is the link (empty when launched from the menu).
pub fn desktop_entry(app: App, vortux_bin: &Path, icon: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name={name}\n\
         Comment={comment}\n\
         Exec={bin} launch {id} %u\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories=Game;\n\
         MimeType=x-scheme-handler/{scheme};\n\
         StartupNotify=false\n",
        name = app.display_name(),
        comment = app.comment(),
        bin = quote_exec_arg(&vortux_bin.to_string_lossy()),
        icon = icon.to_string_lossy(),
        id = app.id(),
        scheme = app.url_scheme(),
    )
}

/// Write the menu entry and make it the default handler for the app's URL scheme.
pub fn register(paths: &Paths, app: App) -> Result<()> {
    fs::create_dir_all(paths.desktop_dir())?;
    let file = paths.desktop_file(app);
    fs::write(
        &file,
        desktop_entry(app, &paths.vortux_bin(), &paths.icon_file(app, 256)),
    )?;

    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let status = Command::new("xdg-mime")
        .args([
            "default",
            &file_name,
            &format!("x-scheme-handler/{}", app.url_scheme()),
        ])
        .status()?;
    if !status.success() {
        return Err(Error::Command(format!(
            "`xdg-mime default` exited with {status}"
        )));
    }
    Ok(())
}

/// Ask the desktop to re-read its menu entries. Best effort.
pub fn refresh_database(paths: &Paths) {
    if let Some(tool) = find_in_path("update-desktop-database") {
        let _ = Command::new(tool).arg(paths.desktop_dir()).status();
    }
}

/// Also register the scheme inside Wine's registry, in case the app looks there.
/// (This mirrors what worked in manual testing.)
pub fn register_wine_protocol(paths: &Paths, app: App) -> Result<()> {
    let exe = paths.exe_path(app);
    let out = wine_command(paths)
        .args(["winepath", "-w"])
        .arg(&exe)
        .output()?;
    if !out.status.success() {
        return Err(Error::Command("`wine winepath -w` failed".into()));
    }
    let win_path = String::from_utf8_lossy(&out.stdout).trim().to_string();

    let scheme = app.url_scheme();
    let key = format!(r"HKCU\Software\Classes\{scheme}");
    let command_key = format!(r"{key}\shell\open\command");
    let open_command = format!("\"{win_path}\" \"%1\"");

    let entries: [(&str, Vec<String>); 3] = [
        (
            key.as_str(),
            vec!["/ve".into(), "/d".into(), format!("URL:{scheme}")],
        ),
        (
            key.as_str(),
            vec![
                "/v".into(),
                "URL Protocol".into(),
                "/d".into(),
                String::new(),
            ],
        ),
        (
            command_key.as_str(),
            vec!["/ve".into(), "/d".into(), open_command],
        ),
    ];
    for (k, extra) in entries {
        let status = wine_command(paths)
            .args(["reg", "add", k])
            .args(&extra)
            .arg("/f")
            .status()?;
        if !status.success() {
            return Err(Error::Command(format!(
                "`wine reg add {k}` exited with {status}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn quotes_exec_args() {
        assert_eq!(quote_exec_arg("/home/a b/vortux"), "\"/home/a b/vortux\"");
        assert_eq!(quote_exec_arg("50%"), "\"50%%\"");
    }

    #[test]
    fn desktop_entries_claim_only_their_own_scheme() {
        let bin = PathBuf::from("/x/vortux");
        let icon = PathBuf::from("/x/icon.png");
        let player = desktop_entry(App::Player, &bin, &icon);
        let studio = desktop_entry(App::Studio, &bin, &icon);
        assert!(player.contains("Exec=\"/x/vortux\" launch player %u"));
        assert!(player.contains("MimeType=x-scheme-handler/vortex;"));
        assert!(studio.contains("Exec=\"/x/vortux\" launch studio %u"));
        assert!(studio.contains("MimeType=x-scheme-handler/vortex-studio;"));
        assert!(!studio.contains("x-scheme-handler/vortex;"));
    }
}
