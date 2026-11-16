[← Back to README](../../README.md) · **English** · [Magyar](../hu/development.md)

# Development

## Building from source

You need Rust (MSVC):

```
cargo build --release
```

The program is `target\release\cybershell.exe`; copy it into the project folder as `CyberShell.exe` (next to `config.json` and the `ui` folder).

Formatting and checking the code (formatting settings are in `rustfmt.toml`):

```
cargo fmt
cargo clippy --release
```

If the project sits in a very long path, the build may fail because of the Windows 260-character limit. In that case set a shorter output folder with the `CARGO_TARGET_DIR` environment variable.

Command-line switches: `--quit`, `--restore`, `--keep-taskbar` (does not hide the taskbar), `--config <file>`, `--exit-after <sec>` (for testing).

## Source code layout

- `ui\hub.html`: the interface skeleton; `ui\core.js`: shared helpers; `ui\parts\*.js`: one file per surface (Icons, panel, frames, settings, Start menu, pop-up, preview, break bar); `ui\hub.js`: the loader that picks one by the address. Texts are in `ui\i18n.js`, line icons in `ui\icons.js`.
- `src\main.rs`: entry point and event loop; `src\web.rs`: serving the interfaces and pop-up windows; `src\lifecycle.rs`: start, shutdown, migration from the old name.
- `src\hub.rs` and `src\hub\`: the central state of the program, by topic: layout (`layout`), Icons (`icons`), panel (`panel`), Claude (`claude`), settings (`settings`), preview (`preview`), pop-up (`flyout`), Start menu (`start`), Health (`health`), theme (`theme`), Windows sync and neon border (`sync`).
- The other modules talk to Windows: bars (`appbar`), windows (`wintrack`), program list (`apps`), Win key (`keyhook`), clipboard (`clipboard`), Spotify (`media`), system (`sys`), start with Windows (`autostart`), configuration (`config`), Claude usage (`usage`), window thumbnails (`peek`), Windows theme, accent color, Windows Terminal and window borders (`winsync`).

Opened in a browser, `preview.html` shows the whole screen with demo data (through a simple local web server, because browsers do not load the embedded parts from `file://` addresses). The layout can be tried too: `preview.html?icons=top&info=bottom` (position of the Icons and the panel). The Start menu and search can be viewed separately: `ui/hub.html?part=launcher&mode=start` and `ui/hub.html?part=launcher&mode=search&q=fenykep`.
