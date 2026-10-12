#!/bin/bash
# Builds dist/Scientific-Calculator-<version>-Linux-x86_64.tar.gz and dist/scientific-calculator_<version>_amd64.deb
# from target/release/scientific-calculator. Run from the repository root after `cargo build --release`.
set -euo pipefail
cd "$(dirname "$0")/../.."
VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')
mkdir -p dist
W=$(mktemp -d)

# --- .deb ---------------------------------------------------------------
D="$W/deb"
install -Dm755 target/release/scientific-calculator "$D/usr/bin/scientific-calculator"
install -Dm644 packaging/linux/scientific-calculator.desktop "$D/usr/share/applications/scientific-calculator.desktop"
install -Dm644 packaging/icon-256.png "$D/usr/share/icons/hicolor/256x256/apps/scientific-calculator.png"
install -Dm644 assets/Inter-LICENSE.txt "$D/usr/share/doc/scientific-calculator/Inter-LICENSE.txt"
SIZE=$(du -sk "$D" | cut -f1)
mkdir -p "$D/DEBIAN"
cat > "$D/DEBIAN/control" <<CTL
Package: scientific-calculator
Version: $VERSION
Section: math
Priority: optional
Architecture: amd64
Depends: libc6, libgl1, libxkbcommon0
Installed-Size: $SIZE
Maintainer: Tiki Tech <rakesh@tikitech.in>
Description: Scientific and graphing calculator
 A clean, minimal calculator with graphing, equations, matrices,
 statistics, unit conversion and number bases.
CTL
dpkg-deb --root-owner-group --build "$D" "dist/scientific-calculator_${VERSION}_amd64.deb" >/dev/null

# --- tar.gz -------------------------------------------------------------
T="$W/Scientific-Calculator-$VERSION"
mkdir -p "$T"
install -m755 target/release/scientific-calculator "$T/scientific-calculator"
install -m644 packaging/linux/scientific-calculator.desktop packaging/icon-256.png README.md "$T/"
cat > "$T/install.sh" <<'INS'
#!/bin/bash
# Installs Scientific Calculator for the current user (no root needed).
set -e
cd "$(dirname "$0")"
install -Dm755 scientific-calculator "$HOME/.local/bin/scientific-calculator"
install -Dm644 icon-256.png "$HOME/.local/share/icons/hicolor/256x256/apps/scientific-calculator.png"
install -Dm644 scientific-calculator.desktop "$HOME/.local/share/applications/scientific-calculator.desktop"
sed -i "s|^Exec=.*|Exec=$HOME/.local/bin/scientific-calculator|" "$HOME/.local/share/applications/scientific-calculator.desktop"
echo "Installed. Find Scientific Calculator in your applications menu, or run: $HOME/.local/bin/scientific-calculator"
INS
chmod +x "$T/install.sh"
tar -C "$W" -czf "dist/Scientific-Calculator-$VERSION-Linux-x86_64.tar.gz" "Scientific-Calculator-$VERSION"
rm -rf "$W"
ls -la dist
