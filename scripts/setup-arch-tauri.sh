#!/usr/bin/env bash
set -euo pipefail

# One-click setup for Tauri (Arch Linux)
# Usage:
#   ./scripts/setup-arch-tauri.sh
#   npm run setup:arch

if ! command -v pacman >/dev/null 2>&1; then
  echo "This script is for Arch Linux (pacman) only." >&2
  exit 1
fi

if [ "${EUID:-$(id -u)}" -eq 0 ]; then
  echo "Please run as a normal user (not root)." >&2
  exit 1
fi

have_pkg() {
  pacman -Si "$1" >/dev/null 2>&1
}

pick_first_pkg() {
  for pkg in "$@"; do
    if have_pkg "$pkg"; then
      echo "$pkg"
      return 0
    fi
  done
  return 1
}

echo "[1/5] Installing system dependencies..."
webkit_pkg="$(pick_first_pkg webkit2gtk-4.1 webkit2gtk || true)"
jscore_pkg="$(pick_first_pkg javascriptcoregtk-4.1 javascriptcoregtk || true)"
indicator_pkg="$(pick_first_pkg libayatana-appindicator libappindicator-gtk3 || true)"

pkgs=(
  base-devel
  curl
  wget
  file
  openssl
  pkgconf
  gcc
  make
  cmake
  ninja
  gtk3
  libsoup3
  glib2
  gdk-pixbuf2
  pango
  cairo
  atk
  at-spi2-core
  librsvg
  xdg-utils
  patchelf
  nodejs
  npm
  rustup
)

if [ -n "${webkit_pkg}" ]; then
  pkgs+=("${webkit_pkg}")
else
  echo "Warning: webkit2gtk package not found in repos." >&2
fi

if [ -n "${jscore_pkg}" ]; then
  pkgs+=("${jscore_pkg}")
else
  echo "Warning: javascriptcoregtk package not found in repos." >&2
fi

if [ -n "${indicator_pkg}" ]; then
  pkgs+=("${indicator_pkg}")
fi

sudo pacman -Syu --needed --noconfirm "${pkgs[@]}"