#!/bin/sh
set -eu
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -x "$APP_DIR/bin/system-monitor" ]; then
    exec "$APP_DIR/bin/system-monitor" "$@"
fi
cd "$APP_DIR"
exec cargo run --locked -- "$@"
