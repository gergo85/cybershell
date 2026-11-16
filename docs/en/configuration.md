[← Back to README](../../README.md) · **English** · [Magyar](../hu/configuration.md)

# Configuration

## Adding programs

Every entry in the `apps` list of `config.json` is one icon:

```json
{ "name": "Lightroom", "path": "C:\\Program Files\\Adobe\\Adobe Lightroom Classic\\Lightroom.exe", "icon": "photo" }
```

| Field | Meaning |
|---|---|
| `name` | The label under the icon (can be hidden in the settings). |
| `path` | Exe, folder or URI. Variables such as `%LOCALAPPDATA%` can be used. For Store apps: `shell:AppsFolder\<AppID>` (the PowerShell command `Get-StartApps` lists the AppIDs). |
| `args`, `cwd` | Start arguments and working folder (optional). |
| `icon` | A built-in icon: `browser`, `globe`, `code`, `folder`, `panels`, `terminal`, `doc`, `sheet`, `music`, `photo`, `enhance`, `design`, `picker`, `video`, `mail`, `chat`, `game`, `rocket`, `notes`, `settings`, `calendar`, `cloud`, `spark`, `app`. An image file also works, e.g. `icons/my.png` (relative to the `ui` folder) or a full path. If left empty, one is chosen from the program name. |
| `match` | Exe names whose windows belong to this icon. Needed when the launcher and the running program have different names (e.g. `wt.exe` → `windowsterminal.exe`). |
| `matchTitle` | Part of the window title, if the program cannot be identified by its exe. |
| `maximize` | With `false` new windows are not maximized (`true` by default). |
| `color` | A subtle icon color that hints at the program, e.g. `"#ff7139"`. |
| `autostart` | With `true` the program starts by itself at sign-in (if CyberShell also starts with Windows). |

Press `Ctrl+Alt+R` after saving.

## More settings

| Field | Meaning |
|---|---|
| `language` | Interface language: `"auto"` (the default: Hungarian on Hungarian Windows, English everywhere else), `"hu"` or `"en"`. The language chosen in the settings overrides the automatic one. The texts are in `ui\i18n.js`; new languages can be added. |
| `startWithWindows` | Whether CyberShell starts at sign-in. The file is authoritative: on start the Windows setting is adjusted to it. |
| `iconsEdge`, `infoEdge` | Position of the Icons and the info panel: `"left"`, `"right"`, `"top"` or `"bottom"` (Icons on the `left` by default). `infoEdge` also accepts `"auto"` (the default): at the `bottom` on laptops (battery-powered computers), where the side would be too short, and on the `right` on desktops. They cannot be the same. The old `swapSides: true` setting is converted to `right`/`left` automatically. |
| `iconLabels` | With `false` the Icons bar shows no program names (`true` by default). |
| `allMonitors` | With `false` CyberShell appears only on the primary screen (on every screen by default). |
| `calendar.feeds` | The attached calendars (name, iCal address, color). They can also be added and removed in the calendar. |
| `sections` | Visibility (`tools`, `claude`, `codex` – `false` by default –, `activity`, `date`, `clock`, `weather`, `music`, `volume`, `power`) and order (`order`) of the info panel sections. Anything missing from the order goes to the end (the Toolbar goes before Claude). |
| `startMenu.winKey` | `true` (the default): the `Win` key opens the CyberShell Start menu and `Win+S` its search. With `false` the Windows ones open. |
| `startMenu.pinned` | IDs of the programs pinned to the Start menu (added by pinning with right-click in the menu). |
| `startMenu.webSearch` | The web search address; `{q}` is replaced by the search text (Google by default). |
| `startMenu.clipboard` | `true` (the default): clipboard history in search. With `false` the program does not watch the clipboard. |
| `leftWidth`, `rightWidth` | Thickness of the Icons and the info panel in pixels (width at the side, height at the top or bottom). |
| `frame.top`, `frame.bottom`, `frame.left`, `frame.right` | Thickness of the thin frames on each edge (where there is no panel). `0` turns it off. |
| `weather.city` | The city for the weather (Budapest by default; can also be set in the settings). |
| `weather.latitude`, `weather.longitude` | Exact coordinates; if given, they are used instead of the city. |
| `weather.wind` | Whether the Weather section shows the wind (`true` by default). |
| `weather.enabled` | Whether the weather is shown. |
| `activity.minutes` | Default countdown length in minutes (20 by default). The value typed on the panel overrides it. |
| `activity.autoStart` | Whether it starts counting by itself on start. |
| `activity.sound` | Whether it plays a sound when it runs out. |
| `claudeUsage` | Whether Claude usage is shown. |
| `appearance` | The theme: `"dark"` (Cyberpunk 2077), `"light"` (Ghost in the Shell) `"auto"` (light by day, dark at night, by sunrise and sunset at the Weather location) or `"windows"` (follows the Windows light/dark mode, switching right away). |
| `theme` | Colors of the dark theme: main (yellow), accent (cyan), alert (pink), background and text (`text`). |
| `themeLight` | Colors of the light theme with the same keys (teal, steel blue, orange-red, paper-white background, dark text). |
| `health.enabled`, `health.minutes` | Turns the Health break reminder on or off, and its length in minutes (5–480, 90 by default). |
| `wallpaper` | Full path of the wallpaper last chosen in the settings. If you change it in the file, CyberShell sets it as the Windows wallpaper on reload. |
| `neonBorder` | Neon border: `"off"` (default), `"active"` (a frame glowing in the theme’s main color around the foreground program) or `"all"` (the other windows’ borders take the theme color too, dimmer; Windows 11). |
| `windowsSync.darkMode` | When `true`, the Windows dark/light mode (apps and system) switches together with the CyberShell theme. |
| `windowsSync.accent` | When `true`, the Windows accent color becomes the theme’s main color. |
| `windowsSync.terminal` | When `true`, Windows Terminal gets a `CyberShell Dark` and a `CyberShell Light` color scheme, and the default profile switches between them with the theme. |
| `hideTaskbar` | With `false` the Windows taskbar stays visible and the frame sits above it. |

The look lives in the `ui\css` folder, one file per component (feel free to edit them; changes show right after a reload):

| File | Contents |
|---|---|
| `base.css` | Colors and fonts (variables), the shared tile, the neon line of the panels, shared animations. |
| `strip.css` | The thin frames on the screen edges. |
| `icons.css` | The Icons bar. |
| `panel.css` | The info panel sections. |
| `peek.css`, `flyout.css`, `launcher.css`, `health.css` | Window preview, side pop-up (weather, calendar), Start menu and search, the break bar. |
| `settings.css` | The settings page. |
| `horizontal.css` | Horizontal layout of panels at the top or bottom. |
| `light.css` | The light theme (Ghost in the Shell). |

The order of the files matters (`ui\hub.html` loads them in this order): the horizontal layout and the light theme come last because they override the base.
