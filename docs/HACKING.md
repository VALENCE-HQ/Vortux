# Hacking on Vortux

## Which file do I edit?

| To change... | Edit |
|---|---|
| Text, screens, buttons | `ui/index.html` |
| Colors, spacing, animation | `ui/style.css` (colors are variables at the top) |
| What a button does, screen flow | `ui/main.js` |
| The 3D background | `ui/scene.js` |
| Window size, title, resizing | `src-tauri/tauri.conf.json` |
| Commands the window can call | `src-tauri/src/main.rs` |
| Download links, exe names, URL schemes | `crates/vortux-core/src/app.rs` |
| Wine and winetricks downloads | `crates/vortux-core/src/wine.rs` |
| Install steps and their order | `crates/vortux-core/src/install.rs` |
| Where files are stored | `crates/vortux-core/src/paths.rs` |
| Icons | `crates/vortux-core/assets/` (menu) and `src-tauri/icons/` (the app) |

## The edit loop

`cargo tauri dev` rebuilds on Rust changes. The first build takes several minutes; after that only changed
code is rebuilt. For UI work, serve the `ui` folder and use a browser instead (see CONTRIBUTING.md).

Right-click in the window and choose **Inspect Element** to see JavaScript errors.

## Calling Rust from the window

```rust
// src-tauri/src/main.rs
#[tauri::command]
fn hello(name: String) -> String {
    format!("Hi {name}")
}
// then add `hello` to tauri::generate_handler![...]
```

```js
// ui/main.js
const greeting = await invoke("hello", { name: "Ker" });
```

To push updates from Rust to the window (this is how the progress bar works), use
`handle.emit("event-name", payload)` and `listen("event-name", (e) => ...)`.

## How a link from the website reaches the game

Each menu entry registers a URL scheme (`vortex://` for Player, `vortex-studio://` for Studio) and runs
`vortux launch <app> %u`. Vortux checks the link belongs to that app, then starts the exe through Wine
with the link as its argument. Errors from this path are written to `~/.local/share/vortux/vortux.log`,
since there's no terminal when the browser starts it.
