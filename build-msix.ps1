# Builds the MSIX package for the Microsoft Store.
# Run from PowerShell in this folder:
#   .\build-msix.ps1          Store package (needs packaging\msix\identity.json filled in from Partner Center)
#   .\build-msix.ps1 -Test    test package with a temporary identity, signed so you can install it on this PC
# The release build comes from build-windows.ps1; run that first, or pass -Build to compile here.
param([switch]$Test, [switch]$Build)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# ---- Identity --------------------------------------------------------------------------------
$id = Get-Content packaging\msix\identity.json -Raw -Encoding UTF8 | ConvertFrom-Json
if ($Test) {
    $id = [pscustomobject]@{
        IdentityName = "SopeTechnology.ScientificCalculator.Test"
        Publisher = "CN=Sope Technology Test"
        PublisherDisplayName = "Sopè Technology"
        DisplayName = "Scientific Calculator (test)"
    }
} elseif (($id.IdentityName + $id.Publisher + $id.DisplayName) -match "PASTE") {
    Write-Host "packaging\msix\identity.json still has placeholders."
    Write-Host "Copy Package/Identity/Name, Package/Identity/Publisher and the reserved app name from"
    Write-Host "Partner Center > your app > Product management > Product identity, then run this again."
    Write-Host "To try the package on this PC before that, run:  .\build-msix.ps1 -Test"
    exit 1
}

# ---- Version: Store packages use four parts and the last must be 0 ----------------------------
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value
$msixVersion = "$version.0"

# ---- The app itself ---------------------------------------------------------------------------
$exe = "target\release\scientific-calculator.exe"
if ($Build -or -not (Test-Path $exe)) {
    Write-Host "Compiling the release build..."
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed. Run .\build-windows.ps1 once to set up Rust, then try again." }
}

# ---- Windows SDK tools (MakeAppx, SignTool) ---------------------------------------------------
function Find-SdkTool($name) {
    $kits = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
    if (-not (Test-Path $kits)) { return $null }
    Get-ChildItem $kits -Directory | Where-Object { $_.Name -match '^10\.' } | Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$name" } | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$makeappx = Find-SdkTool "makeappx.exe"
if (-not $makeappx) {
    Write-Host "MakeAppx comes with the Windows SDK, which is not installed."
    $a = Read-Host "Install the Windows SDK now with winget (about 2 GB)? [Y/n]"
    if ($a -eq "" -or $a -match "^[Yy]") {
        winget install --id Microsoft.WindowsSDK.10.0.26100 -e --accept-source-agreements --accept-package-agreements
        $makeappx = Find-SdkTool "makeappx.exe"
    }
    if (-not $makeappx) { Write-Host "Install the Windows SDK from https://developer.microsoft.com/windows/downloads/windows-sdk/ and run this again."; exit 1 }
}

# ---- Lay out the package ----------------------------------------------------------------------
$stage = "dist\msix-layout"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force "$stage\Assets" | Out-Null
Copy-Item $exe "$stage\ScientificCalculator.exe"
Copy-Item packaging\msix\Assets\*.png "$stage\Assets\"
Copy-Item LICENSE "$stage\LICENSE.txt"

function Escape-Xml($s) { return [System.Security.SecurityElement]::Escape($s) }
$manifest = Get-Content packaging\msix\AppxManifest.xml -Raw -Encoding UTF8
$manifest = $manifest.Replace("{{IDENTITY_NAME}}", (Escape-Xml $id.IdentityName))
$manifest = $manifest.Replace("{{PUBLISHER}}", (Escape-Xml $id.Publisher))
$manifest = $manifest.Replace("{{PUBLISHER_DISPLAY_NAME}}", (Escape-Xml $id.PublisherDisplayName))
$manifest = $manifest.Replace("{{DISPLAY_NAME}}", (Escape-Xml $id.DisplayName))
$manifest = $manifest.Replace("{{VERSION}}", $msixVersion)
[System.IO.File]::WriteAllText((Join-Path $PWD "$stage\AppxManifest.xml"), $manifest, (New-Object System.Text.UTF8Encoding $false))

# ---- Pack -------------------------------------------------------------------------------------
$suffix = if ($Test) { "-test" } else { "" }
$out = "dist\Scientific-Calculator-$version-x64$suffix.msix"
& $makeappx pack /o /d $stage /p $out
if ($LASTEXITCODE -ne 0) { throw "MakeAppx failed (see the message above)" }

if (-not $Test) {
    Write-Host ""
    Write-Host "Done: $PWD\$out"
    Write-Host "Upload it in Partner Center > your app > Packages. Microsoft signs it; no certificate needed."
    exit 0
}

# ---- Test package: sign with a temporary certificate so Windows will install it -----------------
$signtool = Find-SdkTool "signtool.exe"
$cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $id.Publisher } | Select-Object -First 1
if (-not $cert) {
    $cert = New-SelfSignedCertificate -Type Custom -Subject $id.Publisher -KeyUsage DigitalSignature `
        -FriendlyName "Scientific Calculator test signing" -CertStoreLocation Cert:\CurrentUser\My `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
}
& $signtool sign /fd SHA256 /sha1 $cert.Thumbprint $out
if ($LASTEXITCODE -ne 0) { throw "SignTool failed" }

$trusted = Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object { $_.Thumbprint -eq $cert.Thumbprint }
if (-not $trusted) {
    $cer = Join-Path $PWD "dist\test-signing.cer"
    Export-Certificate -Cert $cert -FilePath $cer | Out-Null
    Write-Host "Windows must trust the test certificate once. Approve the administrator prompt."
    Start-Process powershell -Verb RunAs -Wait -ArgumentList "-NoProfile -Command Import-Certificate -FilePath '$cer' -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
}
Write-Host ""
Write-Host "Done: $PWD\$out"
Write-Host "Double-click it to install and try it. Remove it later from Settings > Apps."
