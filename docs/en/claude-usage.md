[← Back to README](../../README.md) · **English** · [Magyar](../hu/claude-usage.md)

# Where Claude usage comes from

It uses three sources, in this order:

1. **Your claude.ai account (recommended, also works with the Claude desktop app).** The **Sign in** button on the panel opens a small window where you sign in to claude.ai. After signing in the window closes by itself, and every five minutes CyberShell fetches the same limits claude.ai shows. The sign-in stays in CyberShell's own browser profile (`%LOCALAPPDATA%\CyberShell\WebView2`); the Claude desktop app is not touched. The sign-in is remembered for 30 days, so you do not need to sign in again after a restart (unless claude.ai itself signs you out). If claude.ai is not reachable at start (e.g. no network yet), the panel shows "Connecting…" and retries within half a minute – it asks you to sign in only when claude.ai really reports a signed-out state. To sign out, delete this folder while CyberShell is not running. The query is unofficial and may stop working after a later claude.ai change.

2. **The Claude Code status line.** After every reply Claude Code passes the 5-hour and weekly limits to the status line. If CyberShell is the status line, it stores them and the panel shows them. Set it up in `%USERPROFILE%\.claude\settings.json`:

   ```json
   "statusLine": { "type": "command", "command": "\"C:\\Users\\<name>\\OneDrive\\Dokumentumok\\GitHub\\cybershell\\CyberShell.exe\" --statusline" }
   ```

   A short line also appears at the bottom of Claude Code in the terminal, for example: `Opus · 5h 12% · hét 40%`. The data reflects the time of the latest Claude Code reply; if the 5-hour window has expired since then, the panel shows "—".

3. **Direct query** with the Claude Code sign-in (`%USERPROFILE%\.claude\.credentials.json`), if the token is stored there in readable form. Newer Claude Code versions no longer store it this way, so this is usually not available.

The second method is not officially documented, so it may stop working after a later change.

CyberShell only reads the sign-in, never refreshes it, and never writes the token anywhere. If the panel says there is no data yet, start a Claude Code session in a terminal (`claude`) and send a message: the panel updates within 30 seconds after the first reply. If you do not need this feature, set `claudeUsage` to `false` in `config.json`.
