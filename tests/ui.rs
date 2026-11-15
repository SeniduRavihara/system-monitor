use slint::{ComponentHandle, ModelRc, VecModel};
use system_monitor::{Monitor, ProcessRow};
#[test]
fn render_monitor_and_confirmation() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            renderer_name: Some("software".into()),
            ..Default::default()
        },
    )))
    .unwrap();
    let ui = Monitor::new().unwrap();
    ui.set_cpu_total("18.4%".into());
    ui.set_memory_total("6.2 GiB / 16 GiB".into());
    ui.set_download_total("4.7 MiB/s".into());
    ui.set_upload_total("192 KiB/s".into());
    ui.set_interface_totals(
        "All-interface counters (separate sample): ↓ 4.8 MiB/s   ↑ 218 KiB/s".into(),
    );
    ui.set_status(
        "Preview data • per-process traffic is collected after administrator authorization".into(),
    );
    ui.set_summary("6 processes • download ↓".into());
    let names = [
        "chrome — utility: network service",
        "crawler-studio",
        "firefox",
        "gnome-shell",
        "system-monitor",
        "pipewire",
    ];
    ui.set_rows(ModelRc::new(VecModel::from(
        names
            .iter()
            .enumerate()
            .map(|(i, n)| ProcessRow {
                pid: 590583 + i as i32,
                name: (*n).into(),
                parent: "chrome · parent 590500".into(),
                cpu: "0.6%".into(),
                memory: "139.8 MiB".into(),
                download: if i == 0 { "4.6 MiB/s" } else { "12.0 KiB/s" }.into(),
                upload: "32.0 KiB/s".into(),
                read: "0 B/s".into(),
                write: "0 B/s".into(),
                selected: i == 0,
                active: true,
            })
            .collect::<Vec<_>>(),
    )));
    ui.set_selected_name("chrome (PID 590583)".into());
    ui.set_details("chrome • PID 590583 • user 1000 • parent chrome (590500)\nExecutable: /opt/google/chrome/chrome\nCommand: /opt/google/chrome/chrome --type=utility --utility-sub-type=network.mojom.NetworkService".into());
    ui.set_can_end(true);
    ui.show().unwrap();
    for (name, w, h, confirm) in [
        ("preview.png", 1240., 820., false),
        ("preview-small.png", 1000., 650., false),
        ("confirmation.png", 1000., 650., true),
    ] {
        ui.window().set_size(slint::LogicalSize::new(w, h));
        ui.set_confirm(confirm);
        let s = ui.window().take_snapshot().unwrap();
        assert_eq!(s.width(), w as u32);
        if let Some(dir) = std::env::var_os("MONITOR_SCREENSHOT_DIR") {
            image::save_buffer(
                std::path::Path::new(&dir).join(name),
                s.as_bytes(),
                s.width(),
                s.height(),
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}
