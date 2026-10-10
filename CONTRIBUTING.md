# Contributing

Bug reports, fixes and ideas are welcome. Vortux is small, so a quick issue or a small pull request is
the best way in.

## Reporting a bug

Open an issue and include your distro, GPU, `wine --version`, and the contents of
`~/.local/share/vortux/vortux.log` if there is one.

## Working on the code

```bash
cargo fmt --all
cargo test
cargo clippy -p vortux-core -p vortux-cli
```

Please run these before opening a pull request. For anything visual you can skip Rust entirely: run
`python3 -m http.server -d ui 8000` and open `http://localhost:8000/`. A mock stands in for the backend, so
the whole wizard works in a browser. Add `?installed` or `?fail=studio` to test those cases.

## Project layout

| Folder | What's in it |
|---|---|
| `crates/vortux-core` | All the real work: system check, Wine prefix, downloads, unpacking, menu entries, uninstall |
| `crates/vortux-cli` | A small terminal front end for the core |
| `src-tauri` | The Tauri app: Rust commands the window can call |
| `ui` | The installer window (plain HTML, CSS and JavaScript, no build step) |

Keep the core free of GUI code. Anything long-running reports through the `Progress` trait, so both
front ends can show it.

## Ground rules

- Don't add Vortex's files to the repo. The installer downloads them.
- Keep it Linux-first and small. If a change needs a new dependency, say why in the pull request.
