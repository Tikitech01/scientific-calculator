#!/usr/bin/env bash
set -euo pipefail

mobile_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sdk_dir="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Library/Android/sdk}}"
ndk_dir="${ANDROID_NDK_HOME:-$(find "$sdk_dir/ndk" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)}"

case "$(uname -s)" in
  Darwin) host_dir=darwin-x86_64 ;;
  Linux) host_dir=linux-x86_64 ;;
  *) echo "Unsupported build host" >&2; exit 1 ;;
esac

linker="$ndk_dir/toolchains/llvm/prebuilt/$host_dir/bin/aarch64-linux-android24-clang"
if [[ ! -x "$linker" ]]; then
  echo "Android NDK linker not found: $linker" >&2
  exit 1
fi

env CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$linker" \
  cargo build --release --target aarch64-linux-android \
  --manifest-path "$mobile_dir/native/Cargo.toml"

library_dir="$mobile_dir/android/app/src/main/jniLibs/arm64-v8a"
mkdir -p "$library_dir"
install -m 644 \
  "$mobile_dir/native/target/aarch64-linux-android/release/libscicalc_mobile_core.so" \
  "$library_dir/libscicalc_mobile_core.so"
