<#
.SYNOPSIS
  MSIX-csomag a CyberShellből, a Microsoft Store-hoz vagy helyi kipróbáláshoz.

.DESCRIPTION
  Lefordítja a programot (cargo build --release), összeállítja a csomag tartalmát a
  target\msix\layout mappában (exe, manifest, ikonok), és a Windows SDK makeappx.exe
  programjával elkészíti a csomagot: target\msix\CyberShell_<verzió>_x64.msix.

  A felület és a gyári háttérképek az exe-be vannak építve, ezért a csomagba csak az exe
  kerül. A beállítások a %LOCALAPPDATA%\CyberShell mappába kerülnek.

  Store-hoz: aláírás nélkül kell feltölteni (a Microsoft írja alá). A -Name, a -Publisher és
  a -PublisherDisplayName értékét a Partner Center adja (Termékkezelés › Termékazonosító).

  Helyi kipróbáláshoz:
   - fejlesztői módban aláírás nélkül is telepíthető a kicsomagolt mappa:
       Add-AppxPackage -Register target\msix\layout\AppxManifest.xml
   - vagy aláírva (-CertThumbprint), egy saját tanúsítvánnyal, aminek a tárgya (Subject)
     megegyezik a -Publisher értékével, és a gép megbízhatónak tartja (TrustedPeople).

.EXAMPLE
  .\packaging\build-msix.ps1

.EXAMPLE
  .\packaging\build-msix.ps1 -Name 12345Szerzo.CyberShell -Publisher "CN=AAAA-BBBB" -PublisherDisplayName "Szerző"
#>
param(
    [string]$Name = 'CyberShell',
    [string]$Publisher = 'CN=CyberShell Dev',
    [string]$PublisherDisplayName = 'CyberShell',
    # négytagú verzió (az utolsó tag a Store-ban 0); alapból a Cargo.toml verziója
    [string]$Version,
    # a meglévő target\release\cybershell.exe-t csomagolja, nem fordít újra
    [switch]$NoBuild,
    # aláírás ezzel a tanúsítvánnyal (ujjlenyomat, a Cert:\CurrentUser\My tárolóból)
    [string]$CertThumbprint
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }

if (-not $Version) {
    $m = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"' | Select-Object -First 1
    if (-not $m) { throw 'A Cargo.toml-ban nem találom a verziót.' }
    $g = $m.Matches[0].Groups
    $Version = '{0}.{1}.{2}.0' -f $g[1].Value, $g[2].Value, $g[3].Value
}
if ($Version -notmatch '^\d+\.\d+\.\d+\.\d+$') { throw "Hibás verzió: $Version (négy szám kell, pl. 2.0.0.0)" }

# a Windows SDK legújabb makeappx.exe-je (és signtool.exe-je)
function Find-SdkTool([string]$exe) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $found = Get-ChildItem -Path $kits -Directory -Filter '10.*' -ErrorAction SilentlyContinue |
        Sort-Object { [version]$_.Name } -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$exe" } |
        Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $found) { throw "Nem találom: $exe. Telepítsd a Windows SDK-t (a Visual Studio Installerből is)." }
    $found
}
$makeappx = Find-SdkTool 'makeappx.exe'

if (-not $NoBuild) {
    Push-Location $root
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw 'A fordítás nem sikerült.' }
    } finally { Pop-Location }
}
$exe = Join-Path $target 'release\cybershell.exe'
if (-not (Test-Path $exe)) { throw "Nincs lefordított program: $exe" }

# a csomag tartalma
$out = Join-Path $target 'msix'
$layout = Join-Path $out 'layout'
if (Test-Path $layout) { Remove-Item $layout -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $layout 'Assets') -Force | Out-Null
Copy-Item $exe (Join-Path $layout 'CyberShell.exe')

# ikonok a program ikonjának 256 pontos képéből
Add-Type -AssemblyName System.Drawing
$ico = New-Object System.Drawing.Icon((Join-Path $root 'assets\cybershell.ico'), 256, 256)
$src = $ico.ToBitmap()
foreach ($i in @(@('Square44x44Logo', 44), @('Square150x150Logo', 150), @('StoreLogo', 50))) {
    $size = $i[1]
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $gr = [System.Drawing.Graphics]::FromImage($bmp)
    $gr.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $gr.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $gr.DrawImage($src, 0, 0, $size, $size)
    $gr.Dispose()
    $bmp.Save((Join-Path $layout "Assets\$($i[0]).png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}
$src.Dispose()
$ico.Dispose()

# a manifest a sablonból
$esc = { param($s) [System.Security.SecurityElement]::Escape($s) }
$manifest = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'AppxManifest.xml')).
    Replace('{{NAME}}', (& $esc $Name)).
    Replace('{{PUBLISHER}}', (& $esc $Publisher)).
    Replace('{{PUBLISHER_DISPLAY_NAME}}', (& $esc $PublisherDisplayName)).
    Replace('{{VERSION}}', $Version)
[IO.File]::WriteAllText((Join-Path $layout 'AppxManifest.xml'), $manifest, (New-Object System.Text.UTF8Encoding $false))

# a csomag (a makeappx a manifestet is ellenőrzi)
$msix = Join-Path $out "CyberShell_${Version}_x64.msix"
$log = & $makeappx pack /d $layout /p $msix /o 2>&1
if ($LASTEXITCODE -ne 0) {
    $log | Write-Host
    throw 'A csomag nem készült el (lásd a makeappx üzenetét fent).'
}

if ($CertThumbprint) {
    $signtool = Find-SdkTool 'signtool.exe'
    & $signtool sign /fd SHA256 /sha1 $CertThumbprint $msix
    if ($LASTEXITCODE -ne 0) { throw 'Az aláírás nem sikerült.' }
}

Write-Host "Kész: $msix"
Write-Host "Verzió: $Version, azonosító: $Name, kiadó: $Publisher"
