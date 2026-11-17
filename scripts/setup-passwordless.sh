#!/usr/bin/env bash
# Script to configure passwordless network monitoring for system-monitor
set -e

echo "=== System Monitor: Setup Passwordless Network Monitoring ==="
echo ""
echo "This script configures NetHogs and/or Polkit so that system-monitor can"
echo "measure per-process network traffic automatically without prompting for a password."
echo ""

NETHOGS_BIN=$(which nethogs 2>/dev/null || true)
if [ -z "$NETHOGS_BIN" ]; then
    echo "NetHogs binary not found. Installing via package manager..."
    if command -v dnf &>/dev/null; then
        sudo dnf install -y nethogs
    elif command -v apt-get &>/dev/null; then
        sudo apt-get update && sudo apt-get install -y nethogs
    else
        echo "Error: Could not install nethogs automatically. Please install it with your package manager." >&2
        exit 1
    fi
    NETHOGS_BIN=$(which nethogs)
fi

echo "1. Granting Linux capabilities to $NETHOGS_BIN..."
sudo setcap "cap_net_admin,cap_net_raw,cap_dac_read_search,cap_sys_ptrace+ep" "$NETHOGS_BIN"

echo "Current capabilities on $NETHOGS_BIN:"
getcap "$NETHOGS_BIN"

echo ""
echo "2. Setting up Polkit rule for system-monitor..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_PATH="$ROOT_DIR/bin/system-monitor"

if [ -d "/etc/polkit-1/rules.d" ]; then
    sudo tee /etc/polkit-1/rules.d/50-system-monitor.rules > /dev/null << EOF
// Allow system-monitor collector to run without password prompt
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.policykit.exec" &&
        action.lookup("program") == "$BIN_PATH") {
        return polkit.Result.YES;
    }
});
EOF
    echo "Installed Polkit rule: /etc/polkit-1/rules.d/50-system-monitor.rules"
fi

echo ""
echo "=== Success! Passwordless network monitoring is now active. ==="
echo "Launch system-monitor and network traffic will stream continuously without password prompts."
