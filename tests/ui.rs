use slint::{ComponentHandle, ModelRc, VecModel};
use system_monitor::{FileSystemRow, Monitor, ProcessRow, generate_donut_path, generate_history_path};

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

    // Populate CPU curves & swatches
    let c1_pts: Vec<f64> = (0..60).map(|i| 75.0 + 10.0 * (i as f64 * 0.2).sin()).collect();
    let c2_pts: Vec<f64> = (0..60).map(|i| 85.0 + 8.0 * (i as f64 * 0.3).cos()).collect();
    let c3_pts: Vec<f64> = (0..60).map(|i| 90.0 + 5.0 * (i as f64 * 0.15).sin()).collect();
    let c4_pts: Vec<f64> = (0..60).map(|i| 80.0 + 12.0 * (i as f64 * 0.25).cos()).collect();
    let c5_pts: Vec<f64> = (0..60).map(|i| 92.0 + 6.0 * (i as f64 * 0.18).sin()).collect();
    let c6_pts: Vec<f64> = (0..60).map(|i| 88.0 + 7.0 * (i as f64 * 0.22).cos()).collect();
    let c7_pts: Vec<f64> = (0..60).map(|i| 91.0 + 5.0 * (i as f64 * 0.12).sin()).collect();
    let c8_pts: Vec<f64> = (0..60).map(|i| 87.0 + 6.0 * (i as f64 * 0.28).cos()).collect();

    ui.set_cpu1_path(generate_history_path(&c1_pts, 100.0).into());
    ui.set_cpu2_path(generate_history_path(&c2_pts, 100.0).into());
    ui.set_cpu3_path(generate_history_path(&c3_pts, 100.0).into());
    ui.set_cpu4_path(generate_history_path(&c4_pts, 100.0).into());
    ui.set_cpu5_path(generate_history_path(&c5_pts, 100.0).into());
    ui.set_cpu6_path(generate_history_path(&c6_pts, 100.0).into());
    ui.set_cpu7_path(generate_history_path(&c7_pts, 100.0).into());
    ui.set_cpu8_path(generate_history_path(&c8_pts, 100.0).into());

    ui.set_cpu1_val("85.7%".into());
    ui.set_cpu2_val("96.8%".into());
    ui.set_cpu3_val("94.4%".into());
    ui.set_cpu4_val("89.7%".into());
    ui.set_cpu5_val("96.1%".into());
    ui.set_cpu6_val("93.7%".into());
    ui.set_cpu7_val("95.2%".into());
    ui.set_cpu8_val("91.3%".into());

    // Memory & Swap
    let mem_pts: Vec<f64> = (0..60).map(|_| 83.7).collect();
    let swap_pts: Vec<f64> = (0..60).map(|_| 75.4).collect();
    ui.set_mem_path(generate_history_path(&mem_pts, 100.0).into());
    ui.set_swap_path(generate_history_path(&swap_pts, 100.0).into());
    ui.set_mem_arc(generate_donut_path(83.7).into());
    ui.set_swap_arc(generate_donut_path(75.4).into());
    ui.set_mem_stat_line1("13.9 GB (83.7%) of 16.6 GB".into());
    ui.set_mem_stat_line2("Cache 4.8 GB".into());
    ui.set_swap_stat_line("22.7 GB (75.4%) of 30.1 GB".into());

    // Network
    let rx_pts: Vec<f64> = (0..60).map(|i| if i > 40 { 10.7 * 1024.0 } else { 0.0 }).collect();
    let tx_pts: Vec<f64> = (0..60).map(|i| if i > 40 { 360.8 * 1024.0 } else { 0.0 }).collect();
    ui.set_net_rx_path(generate_history_path(&rx_pts, 500.0 * 1024.0).into());
    ui.set_net_tx_path(generate_history_path(&tx_pts, 500.0 * 1024.0).into());
    ui.set_net_top_label("500.0 KiB/s".into());
    ui.set_net_mid_label("250.0 KiB/s".into());
    ui.set_net_rx_rate("10.7 KiB/s".into());
    ui.set_net_rx_total("18.0 GiB".into());
    ui.set_net_tx_rate("360.8 KiB/s".into());
    ui.set_net_tx_total("133.9 GiB".into());

    // Disk
    let rd_pts: Vec<f64> = (0..60).map(|i| if i > 40 { 34.2 * 1024.0 * 1024.0 } else { 0.0 }).collect();
    let wr_pts: Vec<f64> = (0..60).map(|i| if i > 40 { 1.5 * 1024.0 * 1024.0 } else { 0.0 }).collect();
    ui.set_disk_read_path(generate_history_path(&rd_pts, 200.0 * 1024.0 * 1024.0).into());
    ui.set_disk_write_path(generate_history_path(&wr_pts, 200.0 * 1024.0 * 1024.0).into());
    ui.set_disk_top_label("200.0 MiB/s".into());
    ui.set_disk_mid_label("100.0 MiB/s".into());
    ui.set_disk_read_rate("34.2 MiB/s".into());
    ui.set_disk_read_total("2.4 TiB".into());
    ui.set_disk_write_rate("1.5 MiB/s".into());
    ui.set_disk_write_total("1.9 TiB".into());

    // Header totals
    ui.set_cpu_total("69.0%".into());
    ui.set_memory_total("7.3 GB".into());
    ui.set_disk_read_header_total("39.8 MiB/s".into());
    ui.set_disk_write_header_total("4.5 MiB/s".into());
    ui.set_download_total("4.7 MiB/s".into());
    ui.set_upload_total("192 KiB/s".into());
    ui.set_interface_totals(
        "All-interface counters (separate sample): ↓ 4.8 MiB/s   ↑ 218 KiB/s".into(),
    );
    ui.set_status("Live network • measured by NetHogs".into());
    ui.set_summary("6 processes • memory ↓".into());

    let names = [
        "agy",
        "chrome",
        "antigravity-ide",
        "gnome-shell",
        "ChatGPT",
        "pipewire",
    ];
    ui.set_rows(ModelRc::new(VecModel::from(
        names
            .iter()
            .enumerate()
            .map(|(i, n)| ProcessRow {
                pid: 2508712 + i as i32,
                name: (*n).into(),
                parent: "systemd · parent 3693".into(),
                cpu: if i == 0 { "15.9 %" } else { "1.0 %" }.into(),
                memory: if i == 0 { "510.4 MB" } else { "397.0 MB" }.into(),
                download: if i == 0 { "4.6 MiB/s" } else { "12.0 KiB/s" }.into(),
                upload: "32.0 KiB/s".into(),
                read: if i == 0 { "397.3 KiB/s" } else { "—" }.into(),
                write: if i == 0 { "902.7 KiB/s" } else { "—" }.into(),
                selected: i == 0,
                active: true,
            })
            .collect::<Vec<_>>(),
    )));
    ui.set_selected_name("agy (PID 2508712)".into());
    ui.set_details("agy • PID 2508712 • user 1000 • parent systemd (3693)\nExecutable: /home/senu/.gemini/antigravity-cli/bin/agy\nCommand: agy".into());
    ui.set_can_end(true);

    // File Systems
    ui.set_filesystems(ModelRc::new(VecModel::from(vec![
        FileSystemRow {
            device: "/dev/nvme0n1p3".into(),
            dir: "/".into(),
            fs_type: "btrfs".into(),
            total: "253.3 GB".into(),
            free: "8.6 GB".into(),
            avail: "8.6 GB".into(),
            used: "96.6% (244.7 GB)".into(),
            used_percent: 96.6,
        },
        FileSystemRow {
            device: "/dev/nvme0n1p3".into(),
            dir: "/home".into(),
            fs_type: "btrfs".into(),
            total: "253.3 GB".into(),
            free: "8.6 GB".into(),
            avail: "8.6 GB".into(),
            used: "96.6% (244.7 GB)".into(),
            used_percent: 96.6,
        },
        FileSystemRow {
            device: "/dev/nvme0n1p2".into(),
            dir: "/boot".into(),
            fs_type: "ext4".into(),
            total: "2.0 GB".into(),
            free: "1.4 GB".into(),
            avail: "1.3 GB".into(),
            used: "28.0% (0.6 GB)".into(),
            used_percent: 28.0,
        },
        FileSystemRow {
            device: "/dev/nvme0n1p1".into(),
            dir: "/boot/efi".into(),
            fs_type: "vfat".into(),
            total: "0.6 GB".into(),
            free: "0.6 GB".into(),
            avail: "0.6 GB".into(),
            used: "3.4% (0.0 GB)".into(),
            used_percent: 3.4,
        },
    ])));

    ui.show().unwrap();

    let save_image = |name: &str, s: &slint::SharedPixelBuffer<slint::Rgba8Pixel>| {
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
    };

    // 1. Resources tab (Tab 0) - Matches Image 0
    ui.set_current_tab(0);
    ui.set_confirm(false);
    ui.window().set_size(slint::LogicalSize::new(1240., 820.));
    let s_res = ui.window().take_snapshot().unwrap();
    save_image("preview-resources.png", &s_res);

    // 2. Processes tab (Tab 1) - Matches Image 1
    ui.set_current_tab(1);
    ui.set_confirm(false);
    let s_proc = ui.window().take_snapshot().unwrap();
    save_image("preview.png", &s_proc);
    save_image("preview-processes.png", &s_proc);

    ui.window().set_size(slint::LogicalSize::new(1000., 650.));
    let s_proc_sm = ui.window().take_snapshot().unwrap();
    save_image("preview-small.png", &s_proc_sm);

    // 3. File Systems tab (Tab 2)
    ui.set_current_tab(2);
    ui.window().set_size(slint::LogicalSize::new(1240., 820.));
    let s_fs = ui.window().take_snapshot().unwrap();
    save_image("preview-filesystems.png", &s_fs);

    // 4. Confirmation dialog
    ui.set_current_tab(1);
    ui.set_confirm(true);
    let s_conf = ui.window().take_snapshot().unwrap();
    save_image("confirmation.png", &s_conf);
}
