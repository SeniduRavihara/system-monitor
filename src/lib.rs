pub mod network;
pub mod process;
slint::include_modules!();
pub fn bytes(n: f64) -> String {
    if n >= 1073741824.0 {
        format!("{:.1} GiB", n / 1073741824.0)
    } else if n >= 1048576.0 {
        format!("{:.1} MiB", n / 1048576.0)
    } else if n >= 1024.0 {
        format!("{:.1} KiB", n / 1024.0)
    } else {
        format!("{n:.0} B")
    }
}
pub fn rate(n: f64) -> String {
    format!("{}/s", bytes(n))
}
