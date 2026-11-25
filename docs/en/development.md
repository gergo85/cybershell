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

### The program (Rust)

- `src\main.rs`: entry point and event loop; background threads report to the main thread with `UserEvent` messages. `src\lifecycle.rs`: start and shutdown. `src\web.rs`: serving the interfaces, their addresses and the shared frame of the pop-up windows.
- `src\hub.rs` and `src\hub\`: the central state of the program (`Hub`). Its fields are grouped into sections by topic, its methods live in one file per topic: layout (`layout`), Icons and launching (`icons`), info panel (`panel`), Recent (`recent`), Claude (`claude`), settings (`settings`), window preview (`preview`), pop-up and calendar (`flyout`), Start menu (`start`), Health and the red warning bars (`health`), desktop menu (`deskmenu`), internet connection (`net`), theme (`theme`), Windows sync and neon border (`sync`).
- The other modules talk to Windows: bars (`appbar`), windows (`wintrack`), program list (`apps`), Win key (`keyhook`), the desktop menu's mouse hook and reading the Windows desktop menu (`deskmenu`), clipboard (`clipboard`), Spotify (`media`), system (`sys`), system monitor (`monitor`), notification area (`tray`), neon border overlay (`neon`), hardware components (`hardware`, `wmi`), Bluetooth device batteries (`periph`), start with Windows (`autostart`), configuration (`config`), Claude and Codex usage (`usage`, `codex`), window thumbnails (`peek`), Windows theme, accent color, Windows Terminal and window borders (`winsync`).
- Shared helper modules: reading and writing the registry (`reg`), converting icons to PNG (`bitmap`), COM on background threads (`com`).

New data for the info panel, on the program side: a `send_…` function (or `…_poll` and `on_…` for a background query) in `src\hub\panel.rs`, and a `Feed` field in the panel data section of `Hub`. The `Feed` keeps the last message sent: an unchanged message is not sent again, and the panel gets it back after a reload. Slow queries (registry, files, device manager) belong on a background thread, like those of the Background apps, Codex and Peripherals modules.

### The interface (HTML, JS, CSS)

- `ui\hub.html`: the interface skeleton, listing every script and stylesheet. `ui\core.js`: shared helpers (`window.HUB`). `ui\hub.js`: the loader that builds the interface from the `part` parameter of the address. `ui\parts\*.js`: one file per surface (Icons, panel, frames, settings, Start menu, pop-up, preview, break bar, desktop menu).
- Texts are in `ui\i18n.js`, line icons in `ui\icons.js`, the shared iCal parser in `ui\ical.js`.
- `ui\modules\`: the info panel modules, one `.js` and one `.css` file each; how to write a new module: `ui\modules\README.md` (Hungarian).

The larger surfaces follow the same pattern: a frame and the parts registered with it. A part is an object: `html()` returns its markup, `setup(ctx)` wires it up and returns hooks (for example `message(msg)` for messages from the program, `demo()` for the preview). The frame only does the shared work and talks to the parts through their hooks.

| Frame | Registration | Parts |
|---|---|---|
| `parts\panel.js` | `HUB.defineModule({…})` | `ui\modules\*.js` |
| `parts\flyout.js` | `HUB.flyViews.push({…})` | `parts\fly-weather.js`, `fly-calendar.js`, `fly-note.js` |
| `parts\settings.js` | `HUB.settingsPanes.push({ id, … })` | `parts\settings-programs.js`, `settings-system.js` |

A new part must also be added to `hub.html`, before the script of its frame. The Panels, Appearance and Features tabs of the settings are still in `settings.js`; if one moves to its own file, it should follow the same pattern. A part only touches its own elements; it reaches the elements of the frame (such as the tab rail) through the functions of `ctx`.

### Preview in the browser

Opened in a browser, `preview.html` shows the whole screen with demo data (through a simple local web server, because browsers do not load the embedded parts from `file://` addresses). The layout can be tried too: `preview.html?icons=top&info=bottom` (position of the Icons and the panel). The Start menu and search can be viewed separately: `ui/hub.html?part=launcher&mode=start` and `ui/hub.html?part=launcher&mode=search&q=fenykep`. The same goes for the views of the pop-up (`ui/hub.html?part=fly&view=calendar`, `events`, `note`; the weather by default) and the tabs of the settings (`ui/hub.html?part=settings&tab=programs`, `panels`, `look`, `features`, `system`).
