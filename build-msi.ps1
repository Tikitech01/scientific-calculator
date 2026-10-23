# Builds the Windows installer as an .msi (for the GitHub release and winget).
# Run from PowerShell in this folder:
#   .\build-msi.ps1          uses the release build from build-windows.ps1 (compiles it if missing)
#   .\build-msi.ps1 -Build   compile again first
# Needs the WiX Toolset v3; the script offers to install it with winget.
param([switch]$Build)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value

# ---- The app itself ---------------------------------------------------------------------------
$exe = "target\release\scientific-calculator.exe"
if ($Build -or -not (Test-Path $exe)) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { $env:Path += ";$env:USERPROFILE\.cargo\bin" }
    Write-Host "Compiling the release build..."
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed. Run .\build-windows.ps1 once to set up Rust, then try again." }
}

# ---- WiX Toolset v3 (candle + light) ----------------------------------------------------------
function Find-Wix {
    $dirs = @()
    if ($env:WIX) { $dirs += (Join-Path $env:WIX "bin") }
    $dirs += Get-ChildItem "${env:ProgramFiles(x86)}" -Directory -Filter "WiX Toolset v3*" -ErrorAction SilentlyContinue | ForEach-Object { Join-Path $_.FullName "bin" }
    $dirs += Get-ChildItem "$env:ProgramFiles" -Directory -Filter "WiX Toolset v3*" -ErrorAction SilentlyContinue | ForEach-Object { Join-Path $_.FullName "bin" }
    foreach ($d in $dirs) { if (Test-Path (Join-Path $d "candle.exe")) { return $d } }
    return $null
}
$wix = Find-Wix
if (-not $wix) {
    Write-Host "The WiX Toolset v3 is needed to build an .msi."
    $a = Read-Host "Install it now with winget (WiXToolset.WiXToolset)? [Y/n]"
    if ($a -eq "" -or $a -match "^[Yy]") {
        winget install --id WiXToolset.WiXToolset -e --accept-source-agreements --accept-package-agreements
        $wix = Find-Wix
    }
    if (-not $wix) { Write-Host "Install WiX v3 from https://wixtoolset.org/docs/wix3/ and run this again."; exit 1 }
}

# ---- Build ------------------------------------------------------------------------------------
New-Item -ItemType Directory -Force dist, dist\msi-obj | Out-Null
$out = "dist\Scientific-Calculator-$version-Windows-x64.msi"
& (Join-Path $wix "candle.exe") -nologo -arch x64 "-dVersion=$version" -ext WixUIExtension -out "dist\msi-obj\main.wixobj" "packaging\msi\main.wxs"
if ($LASTEXITCODE -ne 0) { throw "candle (WiX compile) failed" }
& (Join-Path $wix "light.exe") -nologo -ext WixUIExtension -spdb -out $out "dist\msi-obj\main.wixobj"
if ($LASTEXITCODE -ne 0) { throw "light (WiX link) failed" }
Remove-Item dist\msi-obj -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Done: $PWD\$out"
Write-Host "Double-click to try it (a newer version replaces an older one). Attach it to the GitHub release."
