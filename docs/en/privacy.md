[← Back to README](../../README.md) · **English** · [Magyar](../hu/privacy.md)

# CyberShell Privacy Policy

Last updated: 17 November 2026

CyberShell is a free Windows desktop shell and launcher developed by Szabó Gergő ("the developer"). This policy explains what data the app handles. In short: **CyberShell has no server of its own, does not collect personal data about you, and contains no analytics, telemetry or advertising.**

## Data stored on your computer

Everything the app needs is stored locally and never sent to the developer:

- **Settings** in `config.json` and the `wallpapers` folder, in `%LOCALAPPDATA%\CyberShell`.
- **Sign-in to claude.ai** (optional), in CyberShell's own browser profile in `%LOCALAPPDATA%\CyberShell\WebView2`. You can remove it by deleting that folder while the app is not running.
- **Clipboard history** (last 30 copied texts, optional), kept in memory only and lost on exit. Items that password managers mark as hidden are skipped.
- **Recent files, installed programs and window information** are read from Windows to show the Start menu, search and the Icons bar. They are not stored outside your computer.
- **Usage limits of Claude Code and Codex** (optional) are read from local files of those tools (for example `%USERPROFILE%\.claude` and `%USERPROFILE%\.codex`). CyberShell only reads them and never writes or forwards your sign-in tokens.

## Network connections

CyberShell connects to the internet only for the features below, directly from your computer to the service named. The developer receives none of this data.

| Feature | Service | Data sent |
|---|---|---|
| Weather | Open-Meteo (`api.open-meteo.com`, `geocoding-api.open-meteo.com`) | The city name you set and its coordinates |
| Currency rates | Frankfurter (`api.frankfurter.dev`) | The currencies you selected |
| Calendar (optional) | Google Calendar | The private iCal address you enter yourself, to read your events |
| Claude usage (optional) | claude.ai and `api.anthropic.com` | Your sign-in with claude.ai, to read your usage limits |
| Internet check | Windows network status | Nothing leaves the app |

Each of these services has its own privacy policy, which applies to the data they receive. If you do not use a feature, or turn it off in the settings, the related connection is not made. A search from the Start menu that you send to a web search engine opens in your default browser, and the engine's policy applies.

## What the app does not do

- It does not collect, sell or share personal data.
- It does not use analytics, crash-report uploads, advertising or tracking.
- It does not read your files or clipboard other than to provide the features described above.

## Children

The app is not directed at children and does not knowingly collect any information from anyone.

## Changes

If this policy changes, the new version is published at this address with an updated date.

## Contact

Szabó Gergő · <https://szabogergo.hu/> · Issues: <https://github.com/gergo85/cybershell/issues>
