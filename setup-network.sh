#!/bin/sh
set -eu
if command -v nethogs >/dev/null 2>&1; then
    printf '%s\n' 'NetHogs is already installed. Open System Monitor and enable network monitoring.'
    exit 0
fi
if command -v dnf >/dev/null 2>&1; then
    exec pkexec /usr/bin/dnf install -y nethogs
elif command -v apt-get >/dev/null 2>&1; then
    exec pkexec /usr/bin/apt-get install -y nethogs
else
    printf '%s\n' 'Install nethogs and pkexec with your distribution package manager.' >&2
    exit 1
fi
