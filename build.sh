#!/bin/bash
# Builds "Scientific Calculator.app" in this folder. Run it from Terminal:  ./build.sh
set -euo pipefail
cd "$(dirname "$0")"

APP="Scientific Calculator.app"

# The Rust toolchain may be installed but not yet on PATH in this shell.
if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

if ! xcode-select -p >/dev/null 2>&1; then
    echo "Apple's command line tools are needed to build. Starting their installer…"
    xcode-select --install || true
    echo "When that finishes, run ./build.sh again."
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust is not installed on this Mac."
    read -r -p "Install it now with rustup (the official installer)? [Y/n] " answer
    case "${answer:-Y}" in
        [Yy]*)
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
            ;;
        *)
            echo "Install Rust from https://rustup.rs and run ./build.sh again."
            exit 1
            ;;
    esac
fi

echo "Compiling (the first build takes a few minutes)…"
cargo build --release

# Keep the braces: without them bash reads the "…" as part of the variable name.
echo "Assembling ${APP}…"
rm -rf "${APP}"
mkdir -p "${APP}/Contents/MacOS" "${APP}/Contents/Resources"
cp target/release/scientific-calculator "${APP}/Contents/MacOS/ScientificCalculator"
cp packaging/Info.plist "${APP}/Contents/Info.plist"

ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
    double=$((size * 2))
    sips -z "$size" "$size" packaging/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    sips -z "$double" "$double" packaging/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "${APP}/Contents/Resources/AppIcon.icns"

# Ad-hoc signature so macOS runs the app without complaint on this Mac.
codesign --force --deep --sign - "${APP}" >/dev/null 2>&1 || true

echo
echo "Done: $(pwd)/${APP}"
echo "Open it with:  open \"${APP}\"    (or drag it into Applications)"
