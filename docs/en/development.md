[← Back to README](../../README.md) · **English** · [Magyar](../hu/development.md)

# Development

## Building from source

You need Rust (MSVC):

```
cargo build --release
```

The program is `target\release\cybershell.exe`; copy it into the project folder as `CyberShell.exe` (next to the `ui` folder). It reads its settings from `%LOCALAPPDATA%\CyberShell\config.json`; a `config.json` in the project folder is copied there only once, on first start.

The default wallpapers are the pictures in the project's `wallpapers` folder (jpg, png, bmp): the build puts all of them into the exe, so to replace them, just replace the files there. On start, the program unpacks the missing ones into `%LOCALAPPDATA%\CyberShell\wallpapers`, and rewrites those whose content differs from the built-in picture of the same name; earlier default pictures stay there as your own pictures (Reset moves them to the Recycle Bin).

Formatting and checking the code (formatting settings are in `rustfmt.toml`):

```
cargo fmt
cargo clippy --release
```

If the project sits in a very long path, the build may fail because of the Windows 260-character limit. In that case set a shorter output folder with the `CARGO_TARGET_DIR` environment variable.

Command-line switches: `--quit`, `--restore`, `--keep-taskbar` (does not hide the taskbar), `--config <file>`, `--exit-after <sec>` (for testing).

## MSIX package (Microsoft Store)

The `packaging` folder holds the package manifest (`AppxManifest.xml`) and the script that builds the package. It needs the Windows SDK (`makeappx.exe`):

```
powershell -ExecutionPolicy Bypass -File packaging\build-msix.ps1
```

The script compiles the program, makes the icons from `assets\cybershell.ico`, and writes the package to `target\msix\CyberShell_<version>_x64.msix`. The version comes from `Cargo.toml`. Only the exe goes into the package, because the interface and the default wallpapers are built into it.

- **For the Store**, upload the package unsigned (Microsoft signs it). Take the `-Name`, `-Publisher` and `-PublisherDisplayName` values from Partner Center (Product identity page).
- **To try it locally** with Developer Mode on, register the unpacked folder without signing: `Add-AppxPackage -Register target\msix\layout\AppxManifest.xml`. Remove it with `Get-AppxPackage CyberShell | Remove-AppxPackage`.

When installed as a package, *Start with Windows* uses the package's startup task instead of the `Run` registry key. The task's `TaskId` in the manifest must match `TASK_ID` in `src\autostart.rs`.

## Source code layout

- `ui\hub.html`: the interface skeleton; `ui\core.js`: shared helpers; `ui\parts\*.js`: one file per surface (Icons, panel, frames, settings, Start menu, pop-up, preview, break bar); `ui\hub.js`: the loader that picks one by the address. Texts are in `ui\i18n.js`, line icons in `ui\icons.js`.
- `src\main.rs`: entry point and event loop; `src\web.rs`: serving the interfaces and pop-up windows; `src\lifecycle.rs`: start and shutdown.
- `ui\ical.js`: the shared iCal parser (used by the calendar and the Events module).
- `ui\modules\`: the info panel modules, one `.js` and one `.css` file each; how to write a new module: `ui\modules\README.md` (Hungarian).
- `src\hub.rs` and `src\hub\`: the central state of the program, by topic: layout (`layout`), Icons (`icons`), panel (`panel`), Claude (`claude`), settings (`settings`), preview (`preview`), pop-up (`flyout`), Start menu (`start`), Health (`health`), theme (`theme`), Windows sync and neon border (`sync`).
- The other modules talk to Windows: bars (`appbar`), windows (`wintrack`), program list (`apps`), Win key (`keyhook`), clipboard (`clipboard`), Spotify (`media`), system (`sys`), hardware components (`hardware`, `wmi`), Bluetooth device batteries (`periph`), start with Windows (`autostart`), configuration (`config`), Claude usage (`usage`), window thumbnails (`peek`), Windows theme, accent color, Windows Terminal and window borders (`winsync`).

Opened in a browser, `preview.html` shows the whole screen with demo data (through a simple local web server, because browsers do not load the embedded parts from `file://` addresses). The layout can be tried too: `preview.html?icons=top&info=bottom` (position of the Icons and the panel). The Start menu and search can be viewed separately: `ui/hub.html?part=launcher&mode=start` and `ui/hub.html?part=launcher&mode=search&q=fenykep`.
