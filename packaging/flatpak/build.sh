#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"

echo "=== System Monitor Flatpak Builder ==="

if ! command -v flatpak-builder &>/dev/null; then
    echo "Error: flatpak-builder is not installed."
    echo "Install it with your distribution package manager, e.g.:"
    echo "  sudo dnf install flatpak-builder      # Fedora / RHEL"
    echo "  sudo apt install flatpak-builder      # Ubuntu / Debian"
    echo "  sudo pacman -S flatpak-builder        # Arch"
    exit 1
fi

echo "--> Ensuring Flathub remote is configured..."
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo

echo "--> Installing Flatpak SDK and Rust extension (org.freedesktop.Platform 25.08)..."
flatpak install --user -y flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.rust-stable//25.08

BUILD_DIR="${SCRIPT_DIR}/build-dir"

echo "--> Building Flatpak and installing locally..."
cd "${SCRIPT_DIR}"
flatpak-builder --user --install --force-clean "${BUILD_DIR}" io.github.SeniduRavihara.system_monitor.yml

echo ""
echo "=== Success! ==="
echo "You can now run the Flatpak with:"
echo "  flatpak run io.github.SeniduRavihara.system_monitor"
