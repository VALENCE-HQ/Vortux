<div align=center>
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/3e9f77e9-afcb-4271-b395-f3f84253275b">
  <img width="221" height="243" alt="VTUX" src="https://github.com/user-attachments/assets/3e9f77e9-afcb-4271-b395-f3f84253275b" />
</picture>

---
# Vortux
</div>

An unofficial Linux installer for [Vortex](https://playvortex.io) and Vortex Studio.

Vortex only ships for Windows, so on Linux it runs through Wine. Vortux does the fiddly parts for you:
it sets up a separate Wine environment, downloads the official apps, adds them to your app menu, and
makes the "play" button on the website open the game.

> **Unofficial.** Vortux isn't affiliated with or endorsed by Vortex (the Vortex team knows about it and is fine
> with it). It contains no Vortex files and downloads everything from the official site.

**Status:** early. Tested with Wine 11.0 (Staging) on an AMD GPU (Mesa RADV), on one machine so far. Other
setups may work but haven't been tried, and NVIDIA isn't supported yet.

## What you need

- A working Vulkan driver (`vulkaninfo` should run)
- `xdg-utils`, `tar`, `xz` and `zstd` (most distros have these already)
- Wine is **optional**: Vortux can download its own copy (about 100 MB) and keep it in its own folder, or use
  the Wine you already have. You choose on the first screen.

Missing something? For example:

```bash
sudo pacman -S xdg-utils zstd vulkan-radeon                      # Arch
sudo apt install xdg-utils zstd xz-utils mesa-vulkan-drivers     # Debian / Ubuntu
sudo dnf install xdg-utils zstd xz mesa-vulkan-drivers           # Fedora
```

## Installing

Grab the latest build from the [Releases](../../releases) page (AppImage, `.deb` or `.rpm`), start it, and
follow the steps in the window. Before anything is downloaded, Vortux lists exactly what it is about to fetch.
Afterwards you'll find **Vortux Player** and **Vortux Studio** in your app menu, and the website's play button
will open them.

AppImages need FUSE 2. If yours won't start, install `libfuse2` (Debian / Ubuntu) or `fuse2` (Arch / Fedora).

### About Studio

Vortex only lets logged-in users download Studio, so Vortux can't fetch it by itself. When that happens it shows
the download link and asks you to grab the zip in your browser and drop it onto the window.

## What it connects to

Vortux collects nothing and has no accounts or telemetry. It talks to:

- `playvortex.io` to download the apps
- `github.com` for Wine builds ([Kron4ek/Wine-Builds](https://github.com/Kron4ek/Wine-Builds)), winetricks, and
  the DXVK and VKD3D-Proton releases that winetricks installs

## Building from source

You need Rust and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/#linux).

```bash
cargo install tauri-cli --version "^2.0.0" --locked
./scripts/fetch-three.sh        # downloads three.js for the 3D background
cargo tauri dev                 # run the installer window
cargo tauri build               # AppImage, .deb and .rpm in target/release/bundle/
```

The core logic also has a plain command-line front end that needs no GUI libraries:

```bash
cargo test
cargo run -p vortux-cli -- check
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/HACKING.md](docs/HACKING.md) for how the code is laid out.

## Where things end up

Everything lives under `~/.local/share/vortux/`: the Wine prefix, Vortux's own Wine (if you chose that), the
downloaded apps, logs, and a copy of Vortux itself that the menu entries run. Menu entries and icons go in the
usual `~/.local/share/applications` and `~/.local/share/icons`. The in-app **Uninstall** removes all of it.

If something goes wrong, `vortux.log` and `logs/` in that folder are the first places to look.

## AI Disclosure

Vortux is developed and maintained by me, with AI assistance from Claude (by Anthropic).

## License

[Mozilla Public License 2.0](LICENSE).

The pictures in `ui/assets/community/` are screenshots of community games. They're public domain and used with
the creators' permission.
