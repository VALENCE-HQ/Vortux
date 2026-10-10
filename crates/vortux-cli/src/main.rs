// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::io::Write;
use std::process::ExitCode;

use vortux_core::{system, App, Error, InstallOptions, Paths, Progress, Result, UninstallOptions};

struct CliProgress {
    last_pct: Option<u64>,
}

impl Progress for CliProgress {
    fn step(&mut self, message: &str) {
        println!("==> {message}");
    }

    fn warn(&mut self, message: &str) {
        eprintln!("warning: {message}");
    }

    fn download(&mut self, app: App, done: u64, total: Option<u64>) {
        if let Some(t) = total.filter(|t| *t > 0) {
            let pct = done * 100 / t;
            if self.last_pct != Some(pct) {
                self.last_pct = Some(pct);
                print!(
                    "\r    {}: {pct}% ({} of {} MB)",
                    app.display_name(),
                    done / 1_000_000,
                    t / 1_000_000
                );
                let _ = std::io::stdout().flush();
                if done >= t {
                    println!();
                    self.last_pct = None;
                }
            }
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("check") => cmd_check(),
        Some("install") => cmd_install(&Paths::detect()?, &args[1..]),
        Some("launch") => cmd_launch(&Paths::detect()?, &args[1..]),
        Some("uninstall") => cmd_uninstall(&Paths::detect()?, &args[1..]),
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!(
        "Vortux {} - unofficial Linux bootstrapper for Vortex and Vortex Studio

USAGE:
  vortux check                          check that this system is ready
  vortux install [player] [studio]      install (both by default)
  vortux launch <player|studio> [link]  start an app (menu entries and the browser use this)
  vortux uninstall [--keep-apps] [--keep-prefix]
  vortux help",
        env!("CARGO_PKG_VERSION")
    );
}

fn parse_apps(ids: &[String]) -> Result<Vec<App>> {
    if ids.is_empty() {
        return Ok(App::ALL.to_vec());
    }
    ids.iter()
        .map(|id| {
            App::from_id(id).ok_or_else(|| Error::Other(format!("unknown app '{id}' (use player or studio)")))
        })
        .collect()
}

fn cmd_check() -> Result<()> {
    let r = system::check();
    println!("wine:            {}", r.wine_version.as_deref().unwrap_or("NOT FOUND"));
    println!("winetricks:      {}", if r.winetricks { "found" } else { "not found" });
    println!("xdg-mime:        {}", if r.xdg_mime { "found" } else { "NOT FOUND" });
    println!("vulkan:          {:?}", r.vulkan);
    for w in r.warnings() {
        eprintln!("warning: {w}");
    }
    let problems = r.blocking_problems();
    if problems.is_empty() {
        println!("\nThis system is ready.");
        Ok(())
    } else {
        Err(Error::Preflight(problems))
    }
}

fn cmd_install(paths: &Paths, args: &[String]) -> Result<()> {
    let opts = InstallOptions { apps: parse_apps(args)?, ..Default::default() };
    let mut progress = CliProgress { last_pct: None };
    for app in vortux_core::install(paths, &opts, &mut progress)? {
        eprintln!("{} wasn't installed: the download needs a browser session.", app.display_name());
    }
    println!("\nInstalled. Press play on the Vortex website, or open Vortux Player / Vortux Studio from your app menu.");
    Ok(())
}

fn cmd_launch(paths: &Paths, args: &[String]) -> Result<()> {
    let id = args
        .first()
        .ok_or_else(|| Error::Other("usage: vortux launch <player|studio> [link]".into()))?;
    let app = App::from_id(id).ok_or_else(|| Error::Other(format!("unknown app '{id}'")))?;
    let url = args.get(1).map(String::as_str).filter(|u| !u.is_empty());

    let result = vortux_core::launch(paths, app, url);
    if let Err(e) = &result {
        // When started from the browser there's no terminal, so keep a record.
        vortux_core::log_error(paths, &format!("launch {}: {e}", app.id()));
    }
    result
}

fn cmd_uninstall(paths: &Paths, args: &[String]) -> Result<()> {
    let mut opts = UninstallOptions::default();
    for a in args {
        match a.as_str() {
            "--keep-apps" => opts.remove_apps = false,
            "--keep-prefix" => opts.remove_prefix = false,
            other => return Err(Error::Other(format!("unknown option '{other}'"))),
        }
    }
    let mut progress = CliProgress { last_pct: None };
    vortux_core::uninstall(paths, &opts, &mut progress)
}
