[← Vissza a README-hez](../../README.hu.md) · [English](../en/claude-usage.md) · **Magyar**

# A Claude-használat forrása

Három forrásból dolgozik, ebben a sorrendben:

1. **A claude.ai-fiókod (ajánlott, az asztali Claude-hoz is jó).** A panelen a **Belépés** gombbal megnyílik egy kis ablak, ahol bejelentkezel a claude.ai-ra. Belépés után az ablak magától bezárul, és a CyberShell ötpercenként lekéri ugyanazokat a korlátokat, amelyeket a claude.ai is mutat. A bejelentkezés a CyberShell saját böngészőprofiljában marad (`%LOCALAPPDATA%\CyberShell\WebView2`), az asztali Claude apphoz nem nyúl. A belépést 30 napig megjegyzi, így újraindítás után sem kell újra belépni (hacsak a claude.ai maga ki nem léptet). Ha induláskor a claude.ai még nem érhető el (pl. nincs még hálózat), a panel „Kapcsolódás…” állapotot mutat, és fél percen belül újrapróbálja – belépést csak akkor kér, ha a claude.ai valóban kijelentkezett állapotot jelez. Ha valaha kijelentkeznél, töröld ezt a mappát, amíg a CyberShell nem fut. A lekérdezés nem hivatalos, egy későbbi claude.ai-változás után előfordulhat, hogy nem működik.

2. **A Claude Code állapotsora.** A Claude Code minden válasz után átadja az állapotsornak az 5 órás és a heti korlátot. Ha a CyberShell az állapotsor, ezeket elmenti, és a panel kiírja. Beállítás a `%USERPROFILE%\.claude\settings.json` fájlban:

   ```json
   "statusLine": { "type": "command", "command": "\"C:\\Users\\<név>\\OneDrive\\Dokumentumok\\GitHub\\cybershell\\CyberShell.exe\" --statusline" }
   ```

   Ilyenkor a terminálban a Claude Code alján is megjelenik egy rövid sor, például: `Opus · 5h 12% · hét 40%`. Az adat a legutóbbi Claude Code-válasz idejét tükrözi; ha az 5 órás ablak azóta lejárt, a panel „—” jelet mutat.

3. **Közvetlen lekérdezés** a Claude Code bejelentkezésével (`%USERPROFILE%\.claude\.credentials.json`), ha a token olvasható formában van benne. Az újabb Claude Code-változatok ezt már nem így tárolják, ezért ez többnyire nem elérhető.

A második módszer nem hivatalosan dokumentált, ezért egy későbbi változás után előfordulhat, hogy nem működik.

A CyberShell a bejelentkezést csak olvassa, nem frissíti, és a tokent sehova nem írja ki. Ha a panelen az áll, hogy még nincs adat, indíts el egy terminálban egy Claude Code-munkamenetet (`claude`), és küldj egy üzenetet: az első válasz után a panel 30 másodpercen belül frissül. Ha a funkcióra nincs szükséged, a `config.json`-ban állítsd a `claudeUsage` értékét `false`-ra.
