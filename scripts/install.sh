#!/usr/bin/env bash
# Installation script for System Monitor on Linux
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== System Monitor: Linux Desktop Installer ==="
echo ""

SYSTEM_INSTALL=false
if [ "$1" = "--system" ] || [ "$(id -u)" -eq 0 ]; then
    SYSTEM_INSTALL=true
fi

# 1. Ensure release binary exists and is up to date
BIN_SRC="$ROOT_DIR/bin/system-monitor"
if [ ! -f "$BIN_SRC" ]; then
    echo "Release binary not found at $BIN_SRC. Building now..."
    cd "$ROOT_DIR"
    cargo build --release --locked
    cp "$ROOT_DIR/target/release/system-monitor" "$BIN_SRC"
fi

# 2. Determine target paths
if [ "$SYSTEM_INSTALL" = true ]; then
    BIN_DIR="/usr/local/bin"
    APPS_DIR="/usr/local/share/applications"
    ICON_DIR="/usr/local/share/icons/hicolor/scalable/apps"
    echo "Installing system-wide (requires root)..."
else
    BIN_DIR="$HOME/.local/bin"
    APPS_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
    echo "Installing for current user ($USER)..."
fi

# 3. Create necessary directories
mkdir -p "$BIN_DIR" "$APPS_DIR" "$ICON_DIR"

# 4. Copy binary
echo "• Installing binary to $BIN_DIR/system-monitor"
cp -f "$BIN_SRC" "$BIN_DIR/system-monitor"
chmod 755 "$BIN_DIR/system-monitor"

# 5. Copy icon
echo "• Installing application icon to $ICON_DIR/system-monitor.svg"
cp -f "$ROOT_DIR/assets/icons/system-monitor.svg" "$ICON_DIR/system-monitor.svg"

# 6. Copy and configure desktop entry
echo "• Installing desktop entry to $APPS_DIR/system-monitor.desktop"
cp -f "$ROOT_DIR/assets/system-monitor.desktop" "$APPS_DIR/system-monitor.desktop"

# 7. Update desktop and icon databases
if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache &>/dev/null; then
    gtk-update-icon-cache -f -t "$(dirname "$ICON_DIR")" 2>/dev/null || true
fi

echo ""
echo "=== Success! System Monitor is now installed ==="
echo "• You can launch it from your desktop applications menu / app search as 'System Monitor'."
echo "• Or run it from any terminal simply by typing: system-monitor"
echo ""

# Check for NetHogs capabilities
if command -v getcap &>/dev/null && command -v nethogs &>/dev/null; then
    NETHOGS_BIN=$(which nethogs)
    if getcap "$NETHOGS_BIN" | grep -q "cap_net_raw"; then
        echo "✓ Passwordless network monitoring is already active."
    else
        echo "Tip: To enable passwordless per-process network telemetry without entering passwords,"
        echo "run: sudo $ROOT_DIR/scripts/setup-passwordless.sh"
    fi
fi
