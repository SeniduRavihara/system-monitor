use slint::{ComponentHandle, ModelRc, VecModel};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use system_monitor::{
    network::{self, Network, Traffic},
    process::{self, Sampler, Snapshot},
    *,
};

#[derive(Default)]
pub struct History {
    pub cpu: [VecDeque<f64>; 8],
    pub memory: VecDeque<f64>,
    pub swap: VecDeque<f64>,
    pub net_rx: VecDeque<f64>,
    pub net_tx: VecDeque<f64>,
    pub disk_read: VecDeque<f64>,
    pub disk_write: VecDeque<f64>,
}

#[derive(Default)]
struct View {
    query: String,
    sort: i32,
    ascending: bool,
    active: bool,
    selected: Option<(u32, u64)>,
    tab: i32,
    history: History,
}

fn push_point(buf: &mut VecDeque<f64>, val: f64) {
    buf.push_back(val);
    if buf.len() > 60 {
        buf.pop_front();
    }
}

fn refresh(ui: &Monitor, snapshot: &Snapshot, net: &Network, v: &mut View) {
    let mem_str = if snapshot.memory_used > 0 {
        decimal_bytes(snapshot.memory_used as f64)
    } else if !snapshot.memory.is_empty() {
        snapshot.memory.clone()
    } else {
        "—".into()
    };
    ui.set_cpu_total(format!("{:.1}%", snapshot.cpu).into());
    ui.set_memory_total(mem_str.into());
    ui.set_disk_read_header_total(rate(snapshot.disk_read_rate).into());
    ui.set_disk_write_header_total(rate(snapshot.disk_write_rate).into());
    ui.set_interface_totals(
        format!(
            "All-interface counters (separate sample): ↓ {}   ↑ {}",
            rate(snapshot.down),
            rate(snapshot.up)
        )
        .into(),
    );
    let fresh = net.fresh();
    ui.set_network_running(net.running);
    ui.set_status(if net.running && net.updated.is_some() && !fresh {
        "Network samples are stale — waiting for the collector.".into()
    } else {
        net.status.clone().into()
    });

    // 1. CPU Core histories and swatches
    for i in 0..8 {
        let val = snapshot.cpu_cores.get(i).copied().unwrap_or(0.0);
        push_point(&mut v.history.cpu[i], val);
        let path = generate_history_path(v.history.cpu[i].make_contiguous(), 100.0);
        let val_str = format!("{:.1}%", val);
        match i {
            0 => { ui.set_cpu1_path(path.into()); ui.set_cpu1_val(val_str.into()); }
            1 => { ui.set_cpu2_path(path.into()); ui.set_cpu2_val(val_str.into()); }
            2 => { ui.set_cpu3_path(path.into()); ui.set_cpu3_val(val_str.into()); }
            3 => { ui.set_cpu4_path(path.into()); ui.set_cpu4_val(val_str.into()); }
            4 => { ui.set_cpu5_path(path.into()); ui.set_cpu5_val(val_str.into()); }
            5 => { ui.set_cpu6_path(path.into()); ui.set_cpu6_val(val_str.into()); }
            6 => { ui.set_cpu7_path(path.into()); ui.set_cpu7_val(val_str.into()); }
            7 => { ui.set_cpu8_path(path.into()); ui.set_cpu8_val(val_str.into()); }
            _ => {}
        }
    }

    // 2. Memory & Swap histories and gauges
    let mem_pct = if snapshot.memory_total > 0 {
        (snapshot.memory_used as f64 / snapshot.memory_total as f64 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let swap_pct = if snapshot.swap_total > 0 {
        (snapshot.swap_used as f64 / snapshot.swap_total as f64 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    push_point(&mut v.history.memory, mem_pct);
    push_point(&mut v.history.swap, swap_pct);
    ui.set_mem_path(generate_history_path(v.history.memory.make_contiguous(), 100.0).into());
    ui.set_swap_path(generate_history_path(v.history.swap.make_contiguous(), 100.0).into());
    ui.set_mem_arc(generate_donut_path(mem_pct).into());
    ui.set_swap_arc(generate_donut_path(swap_pct).into());
    ui.set_mem_stat_line1(
        format!(
            "{} ({:.1}%) of {}",
            decimal_bytes(snapshot.memory_used as f64),
            mem_pct,
            decimal_bytes(snapshot.memory_total as f64)
        )
        .into(),
    );
    ui.set_mem_stat_line2(format!("Cache {}", decimal_bytes(snapshot.memory_cached as f64)).into());
    ui.set_swap_stat_line(
        format!(
            "{} ({:.1}%) of {}",
            decimal_bytes(snapshot.swap_used as f64),
            swap_pct,
            decimal_bytes(snapshot.swap_total as f64)
        )
        .into(),
    );

    // 3. Network history & stats
    push_point(&mut v.history.net_rx, snapshot.down);
    push_point(&mut v.history.net_tx, snapshot.up);
    let mut net_peak = 500.0 * 1024.0;
    for &val in v.history.net_rx.iter().chain(v.history.net_tx.iter()) {
        if val > net_peak {
            net_peak = val;
        }
    }
    ui.set_net_rx_path(generate_history_path(v.history.net_rx.make_contiguous(), net_peak).into());
    ui.set_net_tx_path(generate_history_path(v.history.net_tx.make_contiguous(), net_peak).into());
    ui.set_net_top_label(rate(net_peak).into());
    ui.set_net_mid_label(rate(net_peak / 2.0).into());
    ui.set_net_rx_rate(rate(snapshot.down).into());
    ui.set_net_rx_total(bytes(snapshot.net_rx_total as f64).into());
    ui.set_net_tx_rate(rate(snapshot.up).into());
    ui.set_net_tx_total(bytes(snapshot.net_tx_total as f64).into());

    // 4. Disk history & stats
    push_point(&mut v.history.disk_read, snapshot.disk_read_rate);
    push_point(&mut v.history.disk_write, snapshot.disk_write_rate);
    let mut disk_peak = 100.0 * 1024.0 * 1024.0;
    for &val in v.history.disk_read.iter().chain(v.history.disk_write.iter()) {
        if val > disk_peak {
            disk_peak = val;
        }
    }
    ui.set_disk_read_path(generate_history_path(v.history.disk_read.make_contiguous(), disk_peak).into());
    ui.set_disk_write_path(generate_history_path(v.history.disk_write.make_contiguous(), disk_peak).into());
    ui.set_disk_top_label(rate(disk_peak).into());
    ui.set_disk_mid_label(rate(disk_peak / 2.0).into());
    ui.set_disk_read_rate(rate(snapshot.disk_read_rate).into());
    ui.set_disk_read_total(bytes(snapshot.disk_read_total as f64).into());
    ui.set_disk_write_rate(rate(snapshot.disk_write_rate).into());
    ui.set_disk_write_total(bytes(snapshot.disk_write_total as f64).into());

    // 5. File Systems
    let fs_rows: Vec<_> = snapshot
        .filesystems
        .iter()
        .map(|fs| FileSystemRow {
            device: fs.device.clone().into(),
            dir: fs.mount_point.clone().into(),
            fs_type: fs.fs_type.clone().into(),
            total: decimal_bytes(fs.total as f64).into(),
            free: decimal_bytes(fs.free as f64).into(),
            avail: decimal_bytes(fs.avail as f64).into(),
            used: format!("{:.1}% ({})", fs.used_percent, decimal_bytes(fs.used as f64)).into(),
            used_percent: fs.used_percent as f32,
        })
        .collect();
    ui.set_filesystems(ModelRc::new(VecModel::from(fs_rows)));

    // 6. Process table & sorting
    let traffic = |pid: u32| {
        if fresh {
            *net.rows.get(&pid).unwrap_or(&Traffic::default())
        } else {
            Traffic::default()
        }
    };
    let parent = |pid: u32| {
        snapshot
            .processes
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| p.name.as_str())
            .unwrap_or("exited / hidden")
    };
    let mut processes = snapshot.processes.clone();
    if fresh {
        for (&pid, t) in &net.rows {
            if t.up + t.down > 0.0 && !processes.iter().any(|p| p.pid == pid) {
                let mut p = process::Process::default();
                p.pid = pid;
                p.name = if pid == 0 {
                    "Unattributed traffic".into()
                } else {
                    format!("Exited / inaccessible process {pid}")
                };
                processes.push(p);
            }
        }
    }
    let mut list: Vec<_> = processes
        .iter()
        .filter(|p| {
            let t = traffic(p.pid);
            (!v.active || (fresh && t.down + t.up > 0.0))
                && (v.query.is_empty()
                    || format!(
                        "{} {} {} {} {}",
                        p.name,
                        p.pid,
                        p.parent,
                        parent(p.parent),
                        p.command
                    )
                    .to_lowercase()
                    .contains(&v.query))
        })
        .collect();
    let totals = list.iter().fold(Traffic::default(), |mut sum, p| {
        let t = traffic(p.pid);
        sum.up += t.up;
        sum.down += t.down;
        sum
    });
    ui.set_download_total(
        if fresh {
            rate(totals.down)
        } else {
            "—".into()
        }
        .into(),
    );
    ui.set_upload_total(if fresh { rate(totals.up) } else { "—".into() }.into());
    list.sort_by(|a, b| {
        let cmp = match v.sort {
            0 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            1 => a.pid.cmp(&b.pid),
            2 => a.cpu.total_cmp(&b.cpu),
            3 => a.memory.cmp(&b.memory),
            4 => traffic(a.pid).down.total_cmp(&traffic(b.pid).down),
            5 => traffic(a.pid).up.total_cmp(&traffic(b.pid).up),
            6 => a.read.total_cmp(&b.read),
            _ => a.write.total_cmp(&b.write),
        };
        (if v.ascending { cmp } else { cmp.reverse() }).then(a.pid.cmp(&b.pid))
    });
    let sort_name = [
        "name",
        "PID",
        "CPU",
        "memory",
        "download",
        "upload",
        "disk read",
        "disk write",
    ][v.sort as usize];
    ui.set_current_sort(v.sort);
    ui.set_sort_ascending(v.ascending);
    let total_count = list.len();
    ui.set_summary(
        format!(
            "{} rows • {} {}",
            total_count,
            sort_name,
            if v.ascending { "↑" } else { "↓" }
        )
        .into(),
    );
    let rows: Vec<_> = list
        .into_iter()
        .map(|p| {
            let t = traffic(p.pid);
            let known = snapshot.processes.iter().any(|known| known.pid == p.pid);
            ProcessRow {
                pid: p.pid as i32,
                name: p.name.clone().into(),
                parent: if known {
                    format!("{} · parent {}", parent(p.parent), p.parent)
                } else {
                    "Owner unavailable • cannot end this row".into()
                }
                .into(),
                cpu: if known && p.cpu > 0.05 {
                    format!("{:.1} %", p.cpu)
                } else if known {
                    "—".into()
                } else {
                    "—".into()
                }
                .into(),
                memory: if known {
                    decimal_bytes(p.memory as f64)
                } else {
                    "—".into()
                }
                .into(),
                download: if fresh && t.down > 0.0 {
                    rate(t.down)
                } else if fresh {
                    "—".into()
                } else {
                    "—".into()
                }
                .into(),
                upload: if fresh && t.up > 0.0 {
                    rate(t.up)
                } else if fresh {
                    "—".into()
                } else {
                    "—".into()
                }
                .into(),
                read: if p.io.is_some() && p.read > 100.0 {
                    rate(p.read)
                } else {
                    "—".into()
                }
                .into(),
                write: if p.io.is_some() && p.write > 100.0 {
                    rate(p.write)
                } else {
                    "—".into()
                }
                .into(),
                selected: v.selected == Some((p.pid, p.start)),
                active: t.up + t.down > 0.0,
            }
        })
        .collect();
    ui.set_rows(ModelRc::new(VecModel::from(rows)));
    let selected = v.selected.and_then(|(pid, start)| {
        snapshot
            .processes
            .iter()
            .find(|p| p.pid == pid && p.start == start)
    });
    ui.set_can_end(selected.is_some_and(|p| {
        p.uid == unsafe { libc::getuid() } && p.pid > 1 && p.pid != std::process::id()
    }));
    if let Some(p) = selected {
        ui.set_selected_name(format!("{} (PID {})", p.name, p.pid).into());
        ui.set_details(
            format!(
                "{} • PID {} • user {} • parent {} ({})\nExecutable: {}\nCommand: {}",
                p.name,
                p.pid,
                p.uid,
                parent(p.parent),
                p.parent,
                p.exe,
                p.command
            )
            .into(),
        );
    } else {
        ui.set_details(
            if v.selected.is_some() {
                "The selected process has exited."
            } else {
                "Select a process to inspect its executable and full command line."
            }
            .into(),
        );
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("--collector") {
        network::collector()?;
        return Ok(());
    }
    let settings = Rc::new(RefCell::new(AppSettings::load()));
    let ui = Monitor::new()?;

    // Apply remembered window size & maximized state & last tab
    {
        let s = settings.borrow();
        ui.window().set_size(slint::LogicalSize::new(s.width, s.height));
        if s.maximized {
            ui.window().set_maximized(true);
        }
        ui.set_current_tab(s.last_tab);
    }

    let initial_tab = settings.borrow().last_tab;
    let view = Rc::new(RefCell::new(View {
        sort: 3,
        tab: initial_tab,
        ..Default::default()
    }));
    let latest = Arc::new(Mutex::new(Snapshot::default()));
    let net = Arc::new(Mutex::new(Network {
        status: "Network monitoring is off. Enable it to see per-process download and upload."
            .into(),
        ..Default::default()
    }));
    let capture = Rc::new(RefCell::new(None));

    // If NetHogs has Linux capabilities, auto-start network monitoring without password prompt!
    if network::nethogs_has_caps() {
        match network::start(net.clone()) {
            Ok(child) => *capture.borrow_mut() = Some(child),
            Err(e) => net.lock().unwrap().status = e.to_string(),
        }
    }
    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    let data = latest.clone();
    let worker = thread::spawn(move || {
        let mut sampler = Sampler::default();
        while !s.load(Ordering::Relaxed) {
            *data.lock().unwrap() = sampler.sample();
            thread::sleep(Duration::from_secs(1));
        }
    });
    let repaint = {
        let w = ui.as_weak();
        let v = view.clone();
        let l = latest.clone();
        let n = net.clone();
        Rc::new(move || {
            if let Some(ui) = w.upgrade() {
                refresh(&ui, &l.lock().unwrap(), &n.lock().unwrap(), &mut v.borrow_mut());
            }
        })
    };
    {
        let w = ui.as_weak();
        ui.on_minimize_window(move || {
            if let Some(ui) = w.upgrade() {
                ui.window().set_minimized(true);
            }
        });
    }
    {
        let w = ui.as_weak();
        let s = settings.clone();
        ui.on_maximize_window(move || {
            if let Some(ui) = w.upgrade() {
                let max = ui.window().is_maximized();
                let new_max = !max;
                ui.window().set_maximized(new_max);
                s.borrow_mut().maximized = new_max;
                let _ = s.borrow().save();
            }
        });
    }
    {
        let w = ui.as_weak();
        let s = settings.clone();
        ui.on_close_window(move || {
            if let Some(ui) = w.upgrade() {
                let max = ui.window().is_maximized();
                s.borrow_mut().maximized = max;
                if !max {
                    let size = ui.window().size();
                    let scale = ui.window().scale_factor();
                    if scale > 0.0 {
                        let width = size.width as f32 / scale;
                        let height = size.height as f32 / scale;
                        if width >= 400.0 && height >= 300.0 {
                            s.borrow_mut().width = width;
                            s.borrow_mut().height = height;
                        }
                    }
                }
                let _ = s.borrow().save();
            }
            let _ = slint::quit_event_loop();
        });
    }
    {
        let w = ui.as_weak();
        let s = settings.clone();
        ui.window().on_close_requested(move || {
            if let Some(ui) = w.upgrade() {
                let max = ui.window().is_maximized();
                s.borrow_mut().maximized = max;
                if !max {
                    let size = ui.window().size();
                    let scale = ui.window().scale_factor();
                    if scale > 0.0 {
                        let width = size.width as f32 / scale;
                        let height = size.height as f32 / scale;
                        if width >= 400.0 && height >= 300.0 {
                            s.borrow_mut().width = width;
                            s.borrow_mut().height = height;
                        }
                    }
                }
                let _ = s.borrow().save();
            }
            slint::CloseRequestResponse::HideWindow
        });
    }
    {
        let v = view.clone();
        let r = repaint.clone();
        let s = settings.clone();
        ui.on_switch_tab(move |t| {
            v.borrow_mut().tab = t;
            s.borrow_mut().last_tab = t;
            let _ = s.borrow().save();
            r();
        });
    }
    {
        let v = view.clone();
        let r = repaint.clone();
        ui.on_search(move |q| {
            v.borrow_mut().query = q.to_lowercase();
            r();
        });
    }
    {
        let v = view.clone();
        let r = repaint.clone();
        ui.on_active_only(move |a| {
            v.borrow_mut().active = a;
            r();
        });
    }
    {
        let v = view.clone();
        let r = repaint.clone();
        ui.on_sort(move |s| {
            let mut v = v.borrow_mut();
            if v.sort == s {
                v.ascending = !v.ascending
            } else {
                v.sort = s;
                v.ascending = s < 2;
            }
            drop(v);
            r();
        });
    }
    {
        let v = view.clone();
        let l = latest.clone();
        let r = repaint.clone();
        ui.on_select(move |pid| {
            v.borrow_mut().selected = l
                .lock()
                .unwrap()
                .processes
                .iter()
                .find(|p| p.pid == pid as u32)
                .map(|p| (p.pid, p.start));
            r();
        });
    }
    {
        let n = net.clone();
        let c = capture.clone();
        let r = repaint.clone();
        ui.on_network(move || {
            if n.lock().unwrap().running {
                return;
            }
            c.borrow_mut().take();
            match network::start(n.clone()) {
                Ok(child) => *c.borrow_mut() = Some(child),
                Err(e) => n.lock().unwrap().status = e.to_string(),
            }
            r();
        });
    }
    {
        let v = view.clone();
        let w = ui.as_weak();
        ui.on_end_process(move |force| {
            if let Some(ui) = w.upgrade() {
                if let Some((pid, start)) = v.borrow().selected {
                    ui.set_result(
                        match process::end_process(pid, start, force) {
                            Ok(()) => format!(
                                "{} sent to PID {pid}.",
                                if force { "Force quit" } else { "Exit request" }
                            ),
                            Err(e) => format!("Could not end process: {e}"),
                        }
                        .into(),
                    );
                }
            }
        });
    }
    let timer = slint::Timer::default();
    let r = repaint.clone();
    let w_size = ui.as_weak();
    let s_size = settings.clone();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(1000),
        move || {
            r();
            if let Some(ui) = w_size.upgrade() {
                let max = ui.window().is_maximized();
                let mut s = s_size.borrow_mut();
                s.maximized = max;
                if !max {
                    let size = ui.window().size();
                    let scale = ui.window().scale_factor();
                    if scale > 0.0 {
                        let width = size.width as f32 / scale;
                        let height = size.height as f32 / scale;
                        if width >= 400.0
                            && height >= 300.0
                            && ((width - s.width).abs() > 4.0 || (height - s.height).abs() > 4.0)
                        {
                            s.width = width;
                            s.height = height;
                            let _ = s.save();
                        }
                    }
                }
            }
        },
    );
    repaint();
    let result = ui.run();
    let _ = settings.borrow().save();
    stop.store(true, Ordering::Relaxed);
    capture.borrow_mut().take();
    let _ = worker.join();
    result?;
    Ok(())
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use slint::Model;
    use std::time::Instant;
    #[test]
    fn displayed_network_totals_match_process_rows() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Monitor::new().unwrap();
        let snapshot = Snapshot {
            up: 336.9 * 1024.0,
            down: 154.6 * 1024.0,
            processes: vec![
                {
                    let mut p = process::Process::default();
                    p.pid = 101;
                    p.name = "browser".into();
                    p
                },
                {
                    let mut p = process::Process::default();
                    p.pid = 102;
                    p.name = "worker".into();
                    p
                },
            ],
            ..Default::default()
        };
        let mut net = Network {
            running: true,
            updated: Some(Instant::now()),
            rows: [
                (
                    101,
                    Traffic {
                        up: 443.3 * 1024.0,
                        down: 8.8 * 1024.0,
                    },
                ),
                (
                    102,
                    Traffic {
                        up: 341.7 * 1024.0,
                        down: 7.5 * 1024.0,
                    },
                ),
            ]
            .into(),
            ..Default::default()
        };
        let mut v = View::default();
        refresh(&ui, &snapshot, &net, &mut v);
        assert_eq!(ui.get_upload_total().as_str(), rate(785.0 * 1024.0));
        assert_eq!(ui.get_download_total().as_str(), rate(16.3 * 1024.0));
        // Filtering changes the explicitly labelled shown-row totals.
        let mut v_filter = View {
            query: "browser".into(),
            ..Default::default()
        };
        refresh(&ui, &snapshot, &net, &mut v_filter);
        assert_eq!(ui.get_upload_total().as_str(), rate(443.3 * 1024.0));
        // Unknown and departed owners must not silently lose their traffic.
        net.rows.insert(
            0,
            Traffic {
                up: 1024.0,
                down: 2048.0,
            },
        );
        net.rows.insert(
            999,
            Traffic {
                up: 2048.0,
                down: 1024.0,
            },
        );
        let mut v2 = View::default();
        refresh(&ui, &snapshot, &net, &mut v2);
        assert_eq!(ui.get_upload_total().as_str(), rate(788.0 * 1024.0));
        assert_eq!(ui.get_rows().row_count(), 4);
        assert!(!ui.get_can_end());
        // A search with no matches is a live zero, stale data is unavailable.
        let mut v_nomatch = View {
            query: "no-match".into(),
            ..Default::default()
        };
        refresh(&ui, &snapshot, &net, &mut v_nomatch);
        assert_eq!(ui.get_upload_total().as_str(), "0 B/s");
        net.updated = Some(Instant::now() - Duration::from_secs(5));
        let mut v_stale = View::default();
        refresh(&ui, &snapshot, &net, &mut v_stale);
        assert_eq!(ui.get_upload_total().as_str(), "—");
        assert_eq!(ui.get_rows().row_data(0).unwrap().upload.as_str(), "—");
        assert!(ui.get_interface_totals().contains("336.9 KiB/s"));
    }
}
