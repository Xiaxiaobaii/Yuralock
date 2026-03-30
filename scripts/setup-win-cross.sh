#!/usr/bin/env bash
set -euo pipefail

# Install cross toolchain by distro.
if command -v pacman >/dev/null 2>&1; then
  # Arch Linux
  sudo pacman -Syu --needed --noconfirm \
    mingw-w64-gcc \
    mingw-w64-binutils \
    llvm \
    lld \
    pkgconf
elif command -v apt-get >/dev/null 2>&1; then
  # Ubuntu/Debian
  sudo apt-get update
  sudo apt-get install -y \
    mingw-w64 \
    llvm \
    lld \
    pkg-config
else
  echo "Unsupported distro. Please install mingw-w64 toolchain manually." >&2
  exit 1
fi

# Rust targets for GNU Windows toolchains.
rustup target add x86_64-pc-windows-gnu
rustup target add i686-pc-windows-gnu

if ! command -v x86_64-w64-mingw32-dlltool >/dev/null 2>&1; then
  echo "Missing x86_64-w64-mingw32-dlltool. On Arch, ensure mingw-w64-binutils is installed." >&2
  exit 1
fi

echo "Windows cross toolchain is ready."
echo "Build exe: npm run tauri:build:win-exe"
