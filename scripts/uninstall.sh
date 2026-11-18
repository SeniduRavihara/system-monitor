#!/usr/bin/env bash
# Uninstallation script for System Monitor
set -e

echo "=== System Monitor: Linux Desktop Uninstaller ==="
echo ""

SYSTEM_INSTALL=false
if [ "$1" = "--system" ] || [ "$(id -u)" -eq 0 ]; then
    SYSTEM_INSTALL=true
fi

if [ "$SYSTEM_INSTALL" = true ]; then
    BIN_TARGET="/usr/local/bin/system-monitor"
    APP_TARGET="/usr/local/share/applications/system-monitor.desktop"
    ICON_TARGET="/usr/local/share/icons/hicolor/scalable/apps/system-monitor.svg"
    APPS_DIR="/usr/local/share/applications"
else
    BIN_TARGET="$HOME/.local/bin/system-monitor"
    APP_TARGET="$HOME/.local/share/applications/system-monitor.desktop"
    ICON_TARGET="$HOME/.local/share/icons/hicolor/scalable/apps/system-monitor.svg"
    APPS_DIR="$HOME/.local/share/applications"
fi

echo "Removing installed files..."
rm -f "$BIN_TARGET"
rm -f "$APP_TARGET"
rm -f "$ICON_TARGET"

if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi

echo ""
echo "=== System Monitor has been uninstalled. ==="
