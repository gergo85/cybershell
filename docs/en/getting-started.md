[← Back to README](../../README.md) · **English** · [Magyar](../hu/getting-started.md)

# Getting started

## Start and exit

- **Start:** `CyberShell.exe`. The exe works on its own: the interface and the default wallpapers are built in. On first start it creates `config.json` and the `wallpapers` folder in `%LOCALAPPDATA%\CyberShell`. Earlier versions kept these next to the exe: if it finds them there, it copies them over on first start (the originals stay where they are; from then on, edit the new copies). If there is a `ui` folder next to it, that is used instead of the built-in interface (so the look can be edited).
- **Exit:** bottom right **// MENU** → **Exit CyberShell**, or `Ctrl+Alt+Q`, or `CyberShell.exe --quit` from the command line.
- **Reload:** `Ctrl+Alt+R`, or MENU → **Reload**. The program re-reads `config.json` and the interface files.
- **Start with Windows:** the MENU → **Start with Windows** switch. When on, CyberShell starts by itself at sign-in: it waits until Explorer is ready and only then reserves the screen edges. It refreshes the entry on every start, so it still starts from the right place after the folder is moved. It also enables itself in the Windows *Startup apps* list (Task Manager, Settings) – newer Windows 11 builds do not start it otherwise. If you disable it there, CyberShell treats it as off as well.

## Automatic start

Press `Win+R`, type `shell:startup`, and put a shortcut to `CyberShell.exe` into the folder that opens.

## If something goes wrong

If the program stops unexpectedly and the taskbar stays hidden or an empty strip remains at the screen edge, start it like this:

```
CyberShell.exe --restore
```

This restores the taskbar and frees the reserved space. The next normal start does this by itself as well.

Log: `%LOCALAPPDATA%\CyberShell\cybershell.log`, the previous run's is `cybershell.prev.log`. Every click and menu command is logged, so you can see if something never reaches the program.
