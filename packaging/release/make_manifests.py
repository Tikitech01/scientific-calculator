#!/usr/bin/env python3
"""Write package-manager manifests for the files in downloads/.

    python3 packaging/release/make_manifests.py

Each download is referenced through the commit that added it, so a manifest keeps working
after a newer file replaces it on main. Run this after committing new downloads, then commit
the regenerated files under packaging/.

Writes:
  packaging/homebrew/Casks/scientific-calculator.rb   (for a tap: Tikitech01/homebrew-tap)
  packaging/scoop/scientific-calculator.json          (for a Scoop bucket)
  packaging/aur/PKGBUILD and .SRCINFO                 (AUR: scientific-calculator-bin)
  packaging/winget/manifests/.../*.yaml               (pull request to microsoft/winget-pkgs)
"""

import hashlib
import json
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
DL = ROOT / "downloads"
REPO = "Tikitech01/scientific-calculator"
HOME = f"https://github.com/{REPO}"
DESC = "Scientific, graphing, statistics and financial calculator for engineering students"
LONG = (
    "A free, offline scientific calculator: complex numbers and phasors, matrices, equation "
    "solving, calculus, graphing with sliders, unit conversion, hypothesis tests and ANOVA, "
    "finance (EMI, NPV, IRR) and a programmer mode."
)
PUBLISHER = "Sopè Technology"


def sha256(p: pathlib.Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def commit_of(p: pathlib.Path) -> str:
    out = subprocess.run(
        ["git", "log", "-1", "--format=%H", "--", str(p.relative_to(ROOT))],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.strip()
    if not out:
        raise SystemExit(f"{p.name} is not committed yet; commit it first, then run this again.")
    return out


def url(p: pathlib.Path) -> str:
    return f"{HOME}/raw/{commit_of(p)}/downloads/{p.name}"


def find(pattern: str):
    hits = sorted(DL.glob(pattern))
    if not hits:
        return None, None
    p = hits[-1]
    m = re.search(r"(\d+\.\d+\.\d+)", p.name)
    return p, m.group(1)


def write(rel: str, text: str):
    out = ROOT / "packaging" / rel
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text)
    print("wrote", out.relative_to(ROOT))


# ---------------------------------------------------------------- macOS: Homebrew cask (tap)
mac, mac_v = find("Scientific-Calculator-*-macOS-AppleSilicon.zip")
if mac:
    write("homebrew/Casks/scientific-calculator.rb", f'''cask "scientific-calculator" do
  version "{mac_v}"
  sha256 "{sha256(mac)}"

  url "{url(mac)}"
  name "Scientific Calculator"
  desc "{DESC}"
  homepage "{HOME}"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"

  app "Scientific Calculator.app"

  zap trash: [
    "~/Library/Application Support/Scientific Calculator",
  ]

  caveats <<~EOS
    Scientific Calculator is not notarised by Apple yet. If macOS says it cannot be opened,
    open System Settings > Privacy & Security and click "Open Anyway".
  EOS
end
''')

# ---------------------------------------------------------------- Windows: Scoop and winget
win, win_v = find("Scientific-Calculator-*-Windows-Setup.exe")
if win:
    win_sha = sha256(win)
    win_url = url(win)
    write("scoop/scientific-calculator.json", f'''{{
    "version": "{win_v}",
    "description": "{DESC}",
    "homepage": "{HOME}",
    "license": "MIT",
    "architecture": {{
        "64bit": {{
            "url": "{win_url}",
            "hash": "{win_sha}"
        }}
    }},
    "innosetup": true,
    "bin": "ScientificCalculator.exe",
    "shortcuts": [["ScientificCalculator.exe", "Scientific Calculator"]]
}}
''')
    base = f"winget/manifests/s/SopeTechnology/ScientificCalculator/{win_v}"
    ident = "SopeTechnology.ScientificCalculator"
    write(f"{base}/{ident}.yaml", f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.1.12.0.schema.json
PackageIdentifier: {ident}
PackageVersion: {win_v}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.12.0
''')
    write(f"{base}/{ident}.installer.yaml", f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.1.12.0.schema.json
PackageIdentifier: {ident}
PackageVersion: {win_v}
InstallerType: inno
Scope: user
InstallModes:
  - interactive
  - silent
UpgradeBehavior: install
Installers:
  - Architecture: x64
    InstallerUrl: {win_url}
    InstallerSha256: {win_sha.upper()}
ManifestType: installer
ManifestVersion: 1.12.0
''')
    write(f"{base}/{ident}.locale.en-US.yaml", f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.1.12.0.schema.json
PackageIdentifier: {ident}
PackageVersion: {win_v}
PackageLocale: en-US
Publisher: {PUBLISHER}
PublisherUrl: https://sopetechnology.com
PackageName: Scientific Calculator
PackageUrl: {HOME}
License: MIT
LicenseUrl: {HOME}/blob/main/LICENSE
Copyright: "(c) 2026 {PUBLISHER}"
ShortDescription: {json.dumps(DESC)}
Description: {json.dumps(LONG)}
Moniker: scicalc
Tags:
  - calculator
  - scientific-calculator
  - graphing-calculator
  - engineering
  - statistics
  - matrix
  - complex-numbers
ReleaseNotesUrl: {HOME}/blob/main/CHANGELOG.md
ManifestType: defaultLocale
ManifestVersion: 1.12.0
''')

# ---------------------------------------------------------------- Linux: AUR binary package
tar, tar_v = find("Scientific-Calculator-*-Linux-x86_64.tar.gz")
if tar:
    tsha = sha256(tar)
    turl = url(tar)
    lic = ROOT / "LICENSE"
    lsha = sha256(lic)
    lurl = f"{HOME}/raw/{commit_of(lic)}/LICENSE"
    write("aur/PKGBUILD", f'''# Maintainer: {PUBLISHER} <https://sopetechnology.com>
pkgname=scientific-calculator-bin
pkgver={tar_v}
pkgrel=1
pkgdesc="{DESC}"
arch=('x86_64')
url="{HOME}"
license=('MIT')
depends=('libgl' 'libxkbcommon' 'wayland' 'libx11' 'libxcursor' 'libxrandr' 'libxi')
provides=('scientific-calculator')
conflicts=('scientific-calculator')
source=("Scientific-Calculator-$pkgver-Linux-x86_64.tar.gz::{turl}"
        "LICENSE::{lurl}")
sha256sums=('{tsha}'
            '{lsha}')

package() {{
  cd "Scientific-Calculator-$pkgver"
  install -Dm755 scientific-calculator "$pkgdir/usr/bin/scientific-calculator"
  install -Dm644 scientific-calculator.desktop "$pkgdir/usr/share/applications/scientific-calculator.desktop"
  install -Dm644 icon-256.png "$pkgdir/usr/share/icons/hicolor/256x256/apps/scientific-calculator.png"
  install -Dm644 "$srcdir/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}}
''')
    write("aur/.SRCINFO", f'''pkgbase = scientific-calculator-bin
\tpkgdesc = {DESC}
\tpkgver = {tar_v}
\tpkgrel = 1
\turl = {HOME}
\tarch = x86_64
\tlicense = MIT
\tdepends = libgl
\tdepends = libxkbcommon
\tdepends = wayland
\tdepends = libx11
\tdepends = libxcursor
\tdepends = libxrandr
\tdepends = libxi
\tprovides = scientific-calculator
\tconflicts = scientific-calculator
\tsource = Scientific-Calculator-{tar_v}-Linux-x86_64.tar.gz::{turl}
\tsource = LICENSE::{lurl}
\tsha256sums = {tsha}
\tsha256sums = {lsha}

pkgname = scientific-calculator-bin
''')
