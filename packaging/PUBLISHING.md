# Publishing Scientific Calculator

Everything needed to put the app in front of people, in the order that pays off most.

Each release:

```
# 1. put the four files for the version in downloads/
bash packaging/release/upload-r2.sh 3.1.0          # 2. upload to Cloudflare R2 (keys in .env.local)
python3 packaging/release/make_manifests.py        # 3. Homebrew, Scoop, winget, AUR point at R2
```

Downloads are served from
`https://pub-8a5cc98180e2407488e6a7c7e4d86a4b.r2.dev/scientific-calculator/<version>/<file>`;
the README, the website and every manifest link there. Commit what step 3 writes.

## 1. GitHub (do first)

- **Description** (repo ▸ ⚙ next to About):
  `Free, offline scientific calculator for engineering students: phasors, matrices, equations, calculus, graphing, statistics, finance. macOS, Windows, Linux.`
- **Website**: the landing page link
- **Topics**: `calculator scientific-calculator graphing-calculator engineering engineering-students complex-numbers matrix statistics anova financial-calculator rust egui macos windows linux offline`
- **Release**: Releases ▸ Draft a new release ▸ tag `v3.1.0` ▸ title `Scientific Calculator 3.1.0` ▸ paste
  `docs/release-notes/3.1.0.md` ▸ attach the four files from `downloads/` ▸ Publish.
  Release pages show download counts, so you can measure every channel below.
- **Social preview** (Settings ▸ General ▸ Social preview): upload `docs/screenshots/calculate.png`.
  This is the image shown when the repo link is shared on WhatsApp, X, LinkedIn or Slack.

## 2. Windows: winget

Windows users can then run `winget install SopeTechnology.ScientificCalculator`.

1. Put the new Windows installer in `downloads/`, commit, run the script.
2. Fork https://github.com/microsoft/winget-pkgs, copy
   `packaging/winget/manifests/s/SopeTechnology/ScientificCalculator/<version>/` into the same
   path in the fork, open a pull request. Validation runs automatically and takes a day or two.
3. Later versions: same folder with the new version number.

## 3. Windows: Scoop

Live at https://github.com/Tikitech01/scoop-bucket (`bucket/scientific-calculator.json`). After a
new Windows build: run the script, copy the JSON there, commit, push. Users run:

```
scoop bucket add tikitech https://github.com/Tikitech01/scoop-bucket
scoop install scientific-calculator
```

## 4. macOS: Homebrew tap

The main Homebrew catalogue stopped accepting apps that are not notarised by Apple on
1 September 2026, so use your own tap until the app is notarised:

Live at https://github.com/Tikitech01/homebrew-tap. After a new macOS build: run the script, copy
`packaging/homebrew/Casks/scientific-calculator.rb` there, commit, push.
Users run `brew install --cask tikitech01/tap/scientific-calculator`.

Once notarised (Apple Developer Program, USD 99 a year; the release workflow already signs and
notarises when the secrets in the README are set), submit to homebrew/cask for far more reach.

## 5. Linux: AUR

1. Create an account at https://aur.archlinux.org and add your SSH key.
2. `git clone ssh://aur@aur.archlinux.org/scientific-calculator-bin.git`
3. Copy `packaging/aur/PKGBUILD` and `packaging/aur/.SRCINFO` into it, then
   `git add . && git commit -m "Initial release" && git push`.

Arch users then run `yay -S scientific-calculator-bin`.

## 6. Microsoft Store (MSIX)

Submit an MSIX package: Microsoft signs it for free, hosts it and updates users automatically.
(The `.exe` installer route needs a paid code-signing certificate.)

1. Partner Center > Apps and games > New product > MSIX or PWA app > reserve the name.
   Plain "Scientific Calculator" is likely taken; have a distinct name ready.
2. Product management > Product identity: copy Package/Identity/Name, Package/Identity/Publisher
   and the reserved name into `packaging/msix/identity.json`.
3. On the Windows PC: `.\build-msix.ps1` (or double-click `build-msix.bat`). It reuses the release
   build from `build-windows.ps1`, installs the Windows SDK if needed and writes
   `dist\Scientific-Calculator-<version>-x64.msix`.
4. Upload that file in the submission's Packages step; listing text and screenshots go in Store listings.

Before the account is ready, `.\build-msix.ps1 -Test` builds a test package with a temporary
identity and signs it so it installs on that PC.

Icons are in `packaging/msix/Assets` (made from `packaging/icon.png`); the package description is
`packaging/msix/AppxManifest.xml`.

## 6b. Windows: the .msi installer

`build-msi.ps1` (or double-click `build-msi.bat`) on the Windows PC makes
`dist\Scientific-Calculator-<version>-Windows-x64.msi` from the same release build that
`build-windows.ps1` compiles. It uses the WiX Toolset v3 (offered through winget) and
`packaging/msi/main.wxs`: a per-machine install into Program Files, a Start-menu shortcut, the
licence page, and an UpgradeCode that never changes, so installing a newer .msi replaces the
older version in place. Attach the .msi to the GitHub release next to the Inno Setup .exe; winget
accepts either (`InstallerType: wix` for the .msi).

## 7. Other app stores

- **Snap Store**: a `snapcraft.yaml` that repackages the Linux binary; reaches Ubuntu users
  through Ubuntu Software.
- **Flathub**: the MIT licence allows a build from source; needs a Flatpak manifest.

## 8. Licence

The app is released under the MIT License (`LICENSE`, copyright Sopè Technology). The
manifests declare MIT, and the AUR package installs the licence file.
