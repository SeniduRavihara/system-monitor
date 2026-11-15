use std::{
    collections::HashMap,
    fs, io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Instant,
};
#[derive(Clone, Debug, Default)]
pub struct Process {
    pub pid: u32,
    pub parent: u32,
    pub start: u64,
    pub name: String,
    pub command: String,
    pub exe: String,
    pub uid: u32,
    pub memory: u64,
    pub cpu: f64,
    pub read: f64,
    pub write: f64,
    ticks: u64,
    pub io: Option<(u64, u64)>,
}
pub fn parse_stat(text: &str) -> Option<(String, u32, u64, u64, u64)> {
    let left = text.find('(')?;
    let right = text.rfind(')')?;
    let f: Vec<_> = text.get(right + 2..)?.split_whitespace().collect();
    Some((
        text[left + 1..right].into(),
        f.get(1)?.parse().ok()?,
        f.get(19)?.parse().ok()?,
        f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?,
        f.get(21)?.parse::<i64>().ok()?.max(0) as u64,
    ))
}
pub fn read_process(pid: u32) -> Option<Process> {
    let base = format!("/proc/{pid}");
    let (name, parent, start, ticks, rss) =
        parse_stat(&fs::read_to_string(format!("{base}/stat")).ok()?)?;
    let status = fs::read_to_string(format!("{base}/status")).ok()?;
    let uid = status.lines().find_map(|l| {
        l.strip_prefix("Uid:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })?;
    let command = fs::read(format!("{base}/cmdline"))
        .map(|b| String::from_utf8_lossy(&b).replace('\0', " ").trim().into())
        .unwrap_or_default();
    let exe = fs::read_link(format!("{base}/exe"))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "Unavailable".into());
    let io = fs::read_to_string(format!("{base}/io")).ok().and_then(|s| {
        let get = |key: &str| {
            s.lines()
                .find_map(|l| l.strip_prefix(key)?.trim().parse::<u64>().ok())
        };
        Some((get("read_bytes:")?, get("write_bytes:")?))
    });
    Some(Process {
        pid,
        parent,
        start,
        name,
        command,
        exe,
        uid,
        memory: rss * unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64,
        ticks,
        io,
        ..Default::default()
    })
}
#[derive(Default)]
pub struct Sampler {
    previous: HashMap<u32, Process>,
    at: Option<Instant>,
    net: Option<(u64, u64)>,
    cpu: Option<(u64, u64)>,
}
#[derive(Default, Clone)]
pub struct Snapshot {
    pub processes: Vec<Process>,
    pub down: f64,
    pub up: f64,
    pub cpu: f64,
    pub memory: String,
}
impl Sampler {
    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let dt = self
            .at
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(1.0)
            .max(0.001);
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
        let mut next = HashMap::new();
        if let Ok(entries) = fs::read_dir("/proc") {
            for e in entries.flatten() {
                let Some(pid) = e.file_name().to_str().and_then(|s| s.parse().ok()) else {
                    continue;
                };
                let Some(mut p) = read_process(pid) else {
                    continue;
                };
                if let Some(old) = self.previous.get(&pid).filter(|o| o.start == p.start) {
                    p.cpu = p.ticks.saturating_sub(old.ticks) as f64 / hz / dt * 100.0;
                    if let (Some((r, w)), Some((or, ow))) = (p.io, old.io) {
                        p.read = r.saturating_sub(or) as f64 / dt;
                        p.write = w.saturating_sub(ow) as f64 / dt;
                    }
                }
                next.insert(pid, p);
            }
        }
        let mut out = Snapshot::default();
        if let Ok(s) = fs::read_to_string("/proc/net/dev") {
            let (mut rx, mut tx) = (0u64, 0u64);
            for l in s.lines().skip(2) {
                if let Some((name, vals)) = l.split_once(':') {
                    if name.trim() == "lo" {
                        continue;
                    }
                    let v: Vec<u64> = vals
                        .split_whitespace()
                        .filter_map(|v| v.parse().ok())
                        .collect();
                    if v.len() > 8 {
                        rx += v[0];
                        tx += v[8];
                    }
                }
            }
            if let Some((r, t)) = self.net {
                out.down = rx.saturating_sub(r) as f64 / dt;
                out.up = tx.saturating_sub(t) as f64 / dt;
            }
            self.net = Some((rx, tx));
        }
        if let Ok(s) = fs::read_to_string("/proc/stat") {
            let v: Vec<u64> = s
                .lines()
                .next()
                .unwrap_or("")
                .split_whitespace()
                .skip(1)
                .take(8)
                .filter_map(|v| v.parse().ok())
                .collect();
            if v.len() >= 5 {
                let total = v.iter().sum::<u64>();
                let idle = v[3] + v[4];
                if let Some((t, i)) = self.cpu {
                    let d = total.saturating_sub(t);
                    if d > 0 {
                        out.cpu = 100.0 * (1.0 - idle.saturating_sub(i) as f64 / d as f64);
                    }
                }
                self.cpu = Some((total, idle));
            }
        }
        if let Ok(s) = fs::read_to_string("/proc/meminfo") {
            let get = |key: &str| {
                s.lines()
                    .find_map(|l| {
                        l.strip_prefix(key)?
                            .split_whitespace()
                            .next()?
                            .parse::<u64>()
                            .ok()
                    })
                    .unwrap_or(0)
                    * 1024
            };
            let total = get("MemTotal:");
            let used = total.saturating_sub(get("MemAvailable:"));
            out.memory = format!(
                "{} / {}",
                crate::bytes(used as f64),
                crate::bytes(total as f64)
            );
        }
        out.processes = next.values().cloned().collect();
        self.previous = next;
        self.at = Some(now);
        out
    }
}
// A pidfd pins the process identity across the final check and signal.
pub fn end_process(pid: u32, start: u64, force: bool) -> io::Result<()> {
    if pid <= 1 || pid == std::process::id() {
        return Err(io::Error::other("This process is protected"));
    }
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let p = read_process(pid).ok_or_else(|| io::Error::other("Process already exited"))?;
    if p.start != start {
        return Err(io::Error::other("Process changed; select it again"));
    }
    if p.uid != unsafe { libc::getuid() } {
        return Err(io::Error::other("Only your own processes can be ended"));
    }
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            if force { libc::SIGKILL } else { libc::SIGTERM },
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stat_handles_parentheses() {
        let s = "123 (a (worker)) S 1 0 0 0 0 0 0 0 0 0 15 5 0 0 0 0 0 0 99 1000 3";
        assert_eq!(parse_stat(s), Some(("a (worker)".into(), 1, 99, 20, 3)));
    }
    #[test]
    fn samples_real_process() {
        let mut s = Sampler::default();
        assert!(
            s.sample()
                .processes
                .iter()
                .any(|p| p.pid == std::process::id())
        );
    }
    #[test]
    fn identity_guard_and_termination() {
        let mut c = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let p = read_process(c.id()).unwrap();
        assert!(end_process(p.pid, p.start + 1, false).is_err());
        assert!(c.try_wait().unwrap().is_none());
        end_process(p.pid, p.start, false).unwrap();
        assert!(!c.wait().unwrap().success());
    }
}
