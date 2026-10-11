#!/bin/bash
# Builds dist/Numera-<version>-linux-x86_64.tar.gz and dist/numera_<version>_amd64.deb
# from target/release/numera. Run from the repository root after `cargo build --release`.
set -euo pipefail
cd "$(dirname "$0")/../.."
VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')
mkdir -p dist
W=$(mktemp -d)

# --- .deb ---------------------------------------------------------------
D="$W/deb"
install -Dm755 target/release/numera "$D/usr/bin/numera"
install -Dm644 packaging/linux/numera.desktop "$D/usr/share/applications/numera.desktop"
install -Dm644 packaging/icon-256.png "$D/usr/share/icons/hicolor/256x256/apps/numera.png"
install -Dm644 assets/Inter-LICENSE.txt "$D/usr/share/doc/numera/Inter-LICENSE.txt"
SIZE=$(du -sk "$D" | cut -f1)
mkdir -p "$D/DEBIAN"
cat > "$D/DEBIAN/control" <<CTL
Package: numera
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
dpkg-deb --root-owner-group --build "$D" "dist/numera_${VERSION}_amd64.deb" >/dev/null

# --- tar.gz -------------------------------------------------------------
T="$W/Numera-$VERSION"
mkdir -p "$T"
install -m755 target/release/numera "$T/numera"
install -m644 packaging/linux/numera.desktop packaging/icon-256.png README.md "$T/"
cat > "$T/install.sh" <<'INS'
#!/bin/bash
# Installs Numera for the current user (no root needed).
set -e
cd "$(dirname "$0")"
install -Dm755 numera "$HOME/.local/bin/numera"
install -Dm644 icon-256.png "$HOME/.local/share/icons/hicolor/256x256/apps/numera.png"
install -Dm644 numera.desktop "$HOME/.local/share/applications/numera.desktop"
sed -i "s|^Exec=.*|Exec=$HOME/.local/bin/numera|" "$HOME/.local/share/applications/numera.desktop"
echo "Installed. Find Numera in your applications menu, or run: $HOME/.local/bin/numera"
INS
chmod +x "$T/install.sh"
tar -C "$W" -czf "dist/Numera-$VERSION-linux-x86_64.tar.gz" "Numera-$VERSION"
rm -rf "$W"
ls -la dist
