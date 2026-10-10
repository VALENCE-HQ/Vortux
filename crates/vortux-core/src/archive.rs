// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

fn zip_err(e: zip::result::ZipError) -> Error {
    Error::Other(format!("couldn't read the zip archive: {e}"))
}

/// True if the file starts with the Windows executable magic ("MZ").
pub fn looks_like_exe(path: &Path) -> io::Result<bool> {
    let mut f = File::open(path)?;
    let mut magic = [0u8; 2];
    let n = f.read(&mut magic)?;
    Ok(n == 2 && &magic == b"MZ")
}

/// If every entry sits inside one top-level folder (like `Vortex/Vortex.exe`), return its name.
fn common_root(entries: &[(PathBuf, bool)]) -> Option<OsString> {
    let mut root: Option<OsString> = None;
    for (path, is_dir) in entries {
        let mut comps = path.components();
        let first = comps.next()?.as_os_str().to_os_string();
        // A file directly at the top level means there's no single wrapper folder.
        if comps.next().is_none() && !*is_dir {
            return None;
        }
        match &root {
            None => root = Some(first),
            Some(r) if *r == first => {}
            Some(_) => return None,
        }
    }
    root
}

/// Unpack `zip_path` into `dest`, dropping a single wrapper folder if there is one so the
/// files land directly in `dest`. Paths that would escape `dest` are skipped.
pub fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        Error::Other(format!(
            "the download isn't a Windows exe or a readable zip archive: {e}"
        ))
    })?;

    let mut entries: Vec<(PathBuf, bool)> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(zip_err)?;
        if let Some(name) = entry.enclosed_name() {
            entries.push((name, entry.is_dir()));
        }
    }
    let root = common_root(&entries);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_err)?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let rel = match &root {
            Some(r) => name.strip_prefix(r).unwrap_or(&name).to_path_buf(),
            None => name.clone(),
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out_path = dest.join(&rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&out_path)?;
            io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

/// Find the app's exe under `dir`. Returns a path relative to `dir`.
/// Prefers a file named in `preferred`; otherwise accepts a single .exe.
pub fn find_exe(dir: &Path, preferred: &[&str]) -> Result<PathBuf> {
    let mut found = Vec::new();
    collect_exes(dir, dir, 0, &mut found)?;

    for name in preferred {
        let hit = found.iter().find(|p| {
            p.file_name()
                .map(|f| f.to_string_lossy().eq_ignore_ascii_case(name))
                .unwrap_or(false)
        });
        if let Some(p) = hit {
            return Ok(p.clone());
        }
    }

    match found.len() {
        0 => Err(Error::Other("no .exe file was found after unpacking".into())),
        1 => Ok(found.remove(0)),
        _ => {
            let list: Vec<String> = found.iter().map(|p| p.display().to_string()).collect();
            Err(Error::Other(format!(
                "found several .exe files and couldn't tell which one to run: {}",
                list.join(", ")
            )))
        }
    }
}

fn collect_exes(root: &Path, dir: &Path, depth: u32, out: &mut Vec<PathBuf>) -> Result<()> {
    if depth > 3 {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_exes(root, &path, depth + 1, out)?;
        } else if path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
        {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_a_single_wrapper_folder() {
        let entries = vec![
            (PathBuf::from("Vortex"), true),
            (PathBuf::from("Vortex/Vortex.exe"), false),
        ];
        assert_eq!(common_root(&entries), Some(OsString::from("Vortex")));
    }

    #[test]
    fn keeps_flat_archives_as_they_are() {
        let entries = vec![
            (PathBuf::from("Vortex.exe"), false),
            (PathBuf::from("data/a.bin"), false),
        ];
        assert_eq!(common_root(&entries), None);
    }
}
