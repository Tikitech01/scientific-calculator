# Builds the Numera installer for Windows.
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
if (-not (Have link)) {
    Write-Host "Note: if the build fails with 'link.exe not found', install Visual Studio Build Tools with the 'Desktop development with C++' workload:"
    Write-Host "  winget install Microsoft.VisualStudio.2022.BuildTools --override '--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive'"
}

Write-Host "Compiling (the first build takes a few minutes)..."
cargo build --release
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
    & $iscc "/DAppVersion=$version" "packaging\numera.iss"
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed" }
    Write-Host ""
    Write-Host "Done: $PWD\dist\Numera-$version-Setup.exe"
} else {
    # Fallback: a portable zip.
    $zip = "dist\Numera-$version-portable.zip"
    Copy-Item target\release\numera.exe dist\Numera.exe -Force
    Compress-Archive -Path dist\Numera.exe -DestinationPath $zip -Force
    Write-Host ""
    Write-Host "Inno Setup not available. Portable build: $PWD\$zip"
}
