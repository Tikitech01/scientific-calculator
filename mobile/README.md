# Scientific Calculator Studio Pro for Android

Flutter interface for phones and tablets, backed by the same Rust calculation
engine used by the desktop app. This is an installable ARM64 preview. The
current APK is debug signed; Play Store signing and listing are a separate
release step.

## Use

- **Simple, Semi Pro, Pro** share one familiar number pad. Set the view to open
  by default from the view switcher.
- **Math** swaps the pad for a touch editor with fractions, roots, powers,
  parentheses, functions and symbols. **Numbers** restores the number pad
  without losing the expression. **=** evaluates from either keyboard.
- Use the backspace button beside the expression to correct a number, or tap
  a recent calculation to reuse it.
- **Tools** searches workspaces and advanced functions. Graph and Sheet keep
  the calculator expression available when you return.
- **Tools → Shortcuts** controls on-device suggestions. Pin, hide and reset
  affect only the suggestion row; the number pad never changes.

## Build

Requires Flutter, Android SDK/NDK, and Rust's `aarch64-linux-android` target.
From this directory:

```sh
rustup target add aarch64-linux-android
./scripts/build_native_android.sh
flutter pub get
flutter build apk --debug --split-per-abi --target-platform android-arm64
```

Install `build/app/outputs/flutter-apk/app-arm64-v8a-debug.apk` on an ARM64
Android device or emulator. Run `flutter analyze` and `flutter test` for Dart
checks, and `cargo test --manifest-path native/Cargo.toml` for the engine.
Rebuild the Rust library after changing a shared desktop engine module.
