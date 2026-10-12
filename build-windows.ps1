# Builds the Scientific Calculator installer for Windows.
# Run from PowerShell in this folder:   .\build-windows.ps1
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Have($name) { return [bool](Get-Command $name -ErrorAction SilentlyContinue) }

# Rust (the MSVC toolchain also needs Visual Studio Build Tools with "Desktop development with C++").
if (-not (Have cargo)) {
    $env:Path += ";$env:USERPROFILE\.cargo\bin"
}
if (-not (Have cargo)) {
    Write-Host "Rust is not installed."
    $a = Read-Host "Install it now with winget (Rustlang.Rustup)? [Y/n]"
    if ($a -eq "" -or $a -match "^[Yy]") {
        winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
        $env:Path += ";$env:USERPROFILE\.cargo\bin"
        rustup default stable-msvc
    } else { Write-Host "Install Rust from https://rustup.rs and run this again."; exit 1 }
}
# Rust needs a C++ linker on Windows. Use Visual Studio Build Tools when they are installed;
# otherwise offer to install them, or fall back to Rust's GNU toolchain, which brings its own linker.
function Have-MsvcTools {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $path = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return [bool]$path
}

$toolchain = ""
if (-not (Have-MsvcTools)) {
    Write-Host ""
    Write-Host "The Visual Studio C++ build tools are not installed (Rust needs them to link programs)."
    Write-Host "  [1] Install Visual Studio Build Tools now (recommended, about 3 GB download)"
    Write-Host "  [2] Use Rust's GNU toolchain instead (smaller, no Visual Studio needed)"
    $a = Read-Host "Choose 1 or 2 [2]"
    if ($a -eq "1") {
        winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-source-agreements --accept-package-agreements --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --wait"
        if (-not (Have-MsvcTools)) {
            Write-Host "The build tools did not finish installing. Restart the PC if asked, then run this script again."
            exit 1
        }
    } else {
        Write-Host "Installing Rust's GNU toolchain..."
        rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
        if ($LASTEXITCODE -ne 0) { throw "could not install the GNU toolchain" }
        $toolchain = "+stable-x86_64-pc-windows-gnu"
    }
}

Write-Host "Compiling (the first build takes a few minutes)..."
if ($toolchain) { cargo $toolchain build --release } else { cargo build --release }
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value
New-Item -ItemType Directory -Force dist | Out-Null

# Inno Setup makes the installer.
$iscc = @(
    (Get-Command iscc -ErrorAction SilentlyContinue).Source,
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1

if (-not $iscc) {
    Write-Host "Inno Setup is needed to create the installer."
    $a = Read-Host "Install it now with winget (JRSoftware.InnoSetup)? [Y/n]"
    if ($a -eq "" -or $a -match "^[Yy]") {
        winget install --id JRSoftware.InnoSetup -e --accept-source-agreements --accept-package-agreements
        $iscc = @(
            "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
        ) | Where-Object { Test-Path $_ } | Select-Object -First 1
    }
}

if ($iscc) {
    & $iscc "/DAppVersion=$version" "packaging\installer.iss"
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed" }
    Write-Host ""
    Write-Host "Done: $PWD\dist\Scientific-Calculator-$version-Windows-Setup.exe"
} else {
    # Fallback: a portable zip.
    $zip = "dist\Scientific-Calculator-$version-Windows-portable.zip"
    Copy-Item target\release\scientific-calculator.exe dist\ScientificCalculator.exe -Force
    Compress-Archive -Path dist\ScientificCalculator.exe -DestinationPath $zip -Force
    Write-Host ""
    Write-Host "Inno Setup not available. Portable build: $PWD\$zip"
}
