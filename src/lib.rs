pub mod config;
pub mod network;
pub mod process;
slint::include_modules!();

pub use config::AppSettings;

pub fn bytes(n: f64) -> String {
    if n >= 1099511627776.0 {
        format!("{:.1} TiB", n / 1099511627776.0)
    } else if n >= 1073741824.0 {
        format!("{:.1} GiB", n / 1073741824.0)
    } else if n >= 1048576.0 {
        format!("{:.1} MiB", n / 1048576.0)
    } else if n >= 1024.0 {
        format!("{:.1} KiB", n / 1024.0)
    } else {
        format!("{n:.0} B")
    }
}

pub fn decimal_bytes(n: f64) -> String {
    if n >= 1e12 {
        format!("{:.1} TB", n / 1e12)
    } else if n >= 1e9 {
        format!("{:.1} GB", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1} MB", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1} kB", n / 1e3)
    } else {
        format!("{n:.0} bytes")
    }
}

pub fn rate(n: f64) -> String {
    format!("{}/s", bytes(n))
}

pub fn generate_history_path(points: &[f64], max_val: f64) -> String {
    if points.is_empty() {
        return String::new();
    }
    let n = points.len();
    let max = if max_val <= 0.0 { 1.0 } else { max_val };
    let step = 600.0 / 59.0;
    let start_x = 600.0 - (n - 1) as f64 * step;
    let mut cmds = String::with_capacity(n * 18);
    for (i, &val) in points.iter().enumerate() {
        let x = if i == n - 1 {
            600.0
        } else {
            (start_x + i as f64 * step).max(0.0)
        };
        let y = (100.0 - (val / max * 100.0)).clamp(0.0, 100.0);
        if i == 0 {
            cmds.push_str(&format!("M {x:.1} {y:.1}"));
        } else {
            cmds.push_str(&format!(" L {x:.1} {y:.1}"));
        }
    }
    cmds
}

pub fn generate_donut_path(percent: f64) -> String {
    if percent <= 0.1 {
        return String::new();
    }
    if percent >= 99.9 {
        return "M 18 6 A 12 12 0 1 1 17.99 6 Z".into();
    }
    let a = (percent.clamp(0.0, 100.0) / 100.0) * 2.0 * std::f64::consts::PI;
    let end_x = 18.0 + 12.0 * a.sin();
    let end_y = 18.0 - 12.0 * a.cos();
    let large = if percent > 50.0 { 1 } else { 0 };
    format!("M 18 6 A 12 12 0 {large} 1 {end_x:.2} {end_y:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_and_binary_formatting() {
        assert_eq!(decimal_bytes(16.6e9), "16.6 GB");
        assert_eq!(decimal_bytes(4.8e9), "4.8 GB");
        assert_eq!(decimal_bytes(510.4e6), "510.4 MB");
        assert_eq!(bytes(2.4 * 1099511627776.0), "2.4 TiB");
        assert_eq!(rate(0.0), "0 B/s");
        assert_eq!(rate(10.7 * 1024.0), "10.7 KiB/s");
    }

    #[test]
    fn path_generation() {
        let pts: Vec<f64> = (0..60).map(|i| i as f64).collect();
        let path = generate_history_path(&pts, 100.0);
        assert!(path.starts_with("M 0.0 "));
        assert!(path.contains("L 600.0 41.0"));

        let arc = generate_donut_path(50.0);
        assert!(arc.starts_with("M 18 6 A 12 12 0 0 1"));
    }
}
