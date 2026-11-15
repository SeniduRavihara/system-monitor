use slint::{ComponentHandle, ModelRc, VecModel};
use std::{
    cell::RefCell,
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
struct View {
    query: String,
    sort: i32,
    ascending: bool,
    active: bool,
    selected: Option<(u32, u64)>,
}
fn refresh(ui: &Monitor, snapshot: &Snapshot, net: &Network, v: &View) {
    ui.set_cpu_total(format!("{:.1}%", snapshot.cpu).into());
    ui.set_memory_total(snapshot.memory.clone().into());
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
    // Keep captured traffic visible even if /proc cannot supply its owner.
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
    // Derive the cards from exactly the sample and filtered rows shown below.
    // Never mix these with independently sampled interface counters.
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
    ui.set_summary(
        format!(
            "{} rows • {} {}",
            list.len(),
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
                cpu: if known {
                    format!("{:.1}%", p.cpu)
                } else {
                    "—".into()
                }
                .into(),
                memory: if known {
                    bytes(p.memory as f64)
                } else {
                    "—".into()
                }
                .into(),
                download: if fresh { rate(t.down) } else { "—".into() }.into(),
                upload: if fresh { rate(t.up) } else { "—".into() }.into(),
                read: if p.io.is_some() {
                    rate(p.read)
                } else {
                    "—".into()
                }
                .into(),
                write: if p.io.is_some() {
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
    let ui = Monitor::new()?;
    let view = Rc::new(RefCell::new(View {
        sort: 4,
        ..Default::default()
    }));
    let latest = Arc::new(Mutex::new(Snapshot::default()));
    let net = Arc::new(Mutex::new(Network {
        status: "Network monitoring is off. Enable it to see per-process download and upload."
            .into(),
        ..Default::default()
    }));
    let capture = Rc::new(RefCell::new(None));
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
                refresh(&ui, &l.lock().unwrap(), &n.lock().unwrap(), &v.borrow());
            }
        })
    };
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
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(1000),
        move || r(),
    );
    repaint();
    let result = ui.run();
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
        refresh(&ui, &snapshot, &net, &View::default());
        assert_eq!(ui.get_upload_total().as_str(), rate(785.0 * 1024.0));
        assert_eq!(ui.get_download_total().as_str(), rate(16.3 * 1024.0));
        // Filtering changes the explicitly labelled shown-row totals.
        refresh(
            &ui,
            &snapshot,
            &net,
            &View {
                query: "browser".into(),
                ..Default::default()
            },
        );
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
        refresh(&ui, &snapshot, &net, &View::default());
        assert_eq!(ui.get_upload_total().as_str(), rate(788.0 * 1024.0));
        assert_eq!(ui.get_rows().row_count(), 4);
        assert!(!ui.get_can_end());
        // A search with no matches is a live zero, stale data is unavailable.
        refresh(
            &ui,
            &snapshot,
            &net,
            &View {
                query: "no-match".into(),
                ..Default::default()
            },
        );
        assert_eq!(ui.get_upload_total().as_str(), "0 B/s");
        net.updated = Some(Instant::now() - Duration::from_secs(5));
        refresh(&ui, &snapshot, &net, &View::default());
        assert_eq!(ui.get_upload_total().as_str(), "—");
        assert_eq!(ui.get_rows().row_data(0).unwrap().upload.as_str(), "—");
        assert!(ui.get_interface_totals().contains("336.9 KiB/s"));
    }
}
