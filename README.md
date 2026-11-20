# System Monitor

[![Flatpak CI & Release](https://github.com/SeniduRavihara/system-monitor/actions/workflows/flatpak.yml/badge.svg)](https://github.com/SeniduRavihara/system-monitor/actions/workflows/flatpak.yml)
[![GitHub Release](https://img.shields.io/github/v/release/SeniduRavihara/system-monitor?color=blue)](https://github.com/SeniduRavihara/system-monitor/releases/latest)

A modern Linux desktop system and process monitor built with Rust + Slint 1.18.1. Features real-time per-core CPU and memory sparkline histories, collapsible resource categories, host file system breakdown, network bandwidth attribution with NetHogs, and full Flatpak container sandboxing.

---

## Installation

### 1. Download & Install via Flatpak (Recommended)

You can download the standalone `.flatpak` bundle directly from the [Latest Release](https://github.com/SeniduRavihara/system-monitor/releases/latest):

```sh
# Install the Flatpak bundle for the current user
flatpak install SystemMonitor.flatpak

# Run System Monitor
flatpak run io.github.SeniduRavihara.SystemMonitor
```

### 2. Native Desktop Installation

To compile and install System Monitor as a first-class desktop application into your desktop app launcher (`/usr/local/bin` or `~/.local/bin`):

```sh
# Install for current user (no root required)
./scripts/install.sh

# Or install system-wide for all users
sudo ./scripts/install.sh --system
```

Once installed:
- Search for **"System Monitor"** in your GNOME / desktop applications launcher.
- Or run `system-monitor` from any terminal.

To uninstall anytime:
```sh
./scripts/uninstall.sh
```

---

## Run from Source without installing

Double-click `launch.sh`, or run it in a terminal. To build and run directly with Cargo:

```sh
cargo build --release --locked
./target/release/system-monitor
```

Network monitoring requires NetHogs. For passwordless monitoring without repeated authentication prompts, run:
`sudo ./scripts/setup-passwordless.sh`

Alternatively, run `./scripts/setup-network.sh` to install NetHogs through your desktop authorization dialog.
Alternatively, on this Fedora host:

```sh
sudo dnf install nethogs
```

On Ubuntu / Debian: `sudo apt install nethogs policykit-1`.
Start the app as your normal user, then click **Enable network monitoring**.
The desktop authentication dialog grants access only to the collector helper.
The UI does not run as root. No permanent capabilities or policy rules are added.
Closing the app closes a pipe to the helper, which kills and reaps NetHogs.

## Using it

- The table starts sorted by download speed. Click any column heading to sort;
  click again to reverse. The current sort is shown next to the process count.
- Search by name, PID, parent, or command. Use **Network active only** to find
  current users of your connection.
- Select a process to inspect its parent, executable, and complete command line.
  This helps distinguish a browser from Electron apps and Chromium network helpers.
- **End process** sends SIGTERM. **Force quit** sends SIGKILL. Both require an
  in-app confirmation. Only processes owned by your user can be ended; PID 1 and
  the monitor itself are protected. A Linux pidfd and process start-time check
  prevent accidentally signaling a replacement process that reused the same PID.

## What the measurements mean

CPU, memory, disk counters, and interface totals come from Linux `/proc`.
Per-process CPU uses 100% per logical core, so multi-threaded processes can exceed
100%. The CPU overview is normalized across all cores. Memory is resident memory.
Disk counters are actual storage I/O, not network I/O.

Network attribution uses [NetHogs](https://github.com/raboof/nethogs), a separate
system-installed program, through its documented trace output. The table shows
its measured download/upload rates; samples arrive with about one interval of
latency. A dash means monitoring is unavailable or stale, not zero usage.
Samples older than four seconds are discarded from display. Empty live samples
mean zero traffic observed by the collector for that interval.

The download/upload cards are **SHOWN ROWS** totals, calculated from the same
collector sample as the table. Search and active-only filters also apply to these
totals. Small rounding differences are possible when adding formatted numbers.
Captured traffic whose owner has exited or is inaccessible stays visible in an
explicit row; PID 0 is labelled **Unattributed traffic**. These rows cannot be ended.
When collection is off or stale, the cards and rows both show a dash.

A separate **All-interface counters (separate sample)** line shows `/proc` totals
for all non-loopback interfaces. It is not the sum of the process table. VPNs,
virtual interfaces, packet overhead, different sampling windows, and traffic
NetHogs cannot attribute can make that line differ from the process totals.
The GNOME panel also samples independently and is not expected to match each frame.
NetHogs attribution is best effort: some UDP, short-lived sockets, containers,
network namespaces, and shared sockets may be unattributed. Chromium often puts
network activity in its network-service process rather than in the renderer tab
that requested it. The parent and command line aid identification; the app does
not classify high traffic as malicious. Ending a helper may affect its parent app
or cause that app to restart the helper.

This version targets Linux with pidfd support (kernel 5.3+). It is not a Windows
port, historical data logger, packet-content inspector, or automatic process killer.

## Architecture

- `ui/app.slint`: dark process table, resource overview, details, confirmation.
- `src/main.rs`: callbacks, filtering, sorting, identity-based selection, UI timer.
- `src/process.rs`: `/proc` sampler and guarded process termination.
- `src/network.rs`: NetHogs trace parser, freshness state, privileged helper lifecycle.
- `src/lib.rs`: UI bindings and display formatting.

No telemetry, remote services, or network payload storage. Sampling runs off the
UI thread. Build dependencies and application dependencies are pinned in Cargo.lock.

## Validate

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

The tests cover process identity, real `/proc` reads, safe termination of a
disposable child, network parsing/units, stale data, and software-rendered desktop
layouts at 1240×820 and 1000×650, including the confirmation dialog.
`assets/screenshots/preview.png` uses clearly labelled sample data; it is not a recording of your traffic.

For a live capture check after installing NetHogs: enable monitoring, start a known
download, confirm that its process receives traffic, end that disposable process,
and verify the row exits. Privileged packet capture must be validated in your
interactive desktop session; parser tests alone do not prove live attribution.

### Validation in this workspace

- All 8 tests passed, including a regression test for mismatched network totals,
  filtering, missing owners, stale samples, and three rendered UI states.
- `cargo fmt --check` passed.
- Clippy could not run because this machine's active Rust toolchain does not
  include the Clippy component.
- Live elevated capture was not tested: NetHogs is not installed. Run the included
  setup script, then authorize collection in the app.


### Network totals correction

Previously the cards used independently sampled interface counters while the
rows used NetHogs, so a single row could exceed the displayed total. A regression
fixture reproduced 785.0 KiB/s across two rows with a 336.9 KiB/s header. The cards
now sum the actual displayed collector rows, and interface counters have their
own clearly labelled line. This corrects the summary comparison; it does not
claim exact agreement with the independently sampled desktop panel.
