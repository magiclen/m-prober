use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::network;

use crate::{terminal::*, CLIArgs, CLICommands};

#[inline]
pub fn handle_network(args: CLIArgs) {
    debug_assert!(matches!(args.command, CLICommands::Network { .. }));

    if let CLICommands::Network {
        plain,
        light,
        monitor,
        unit,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(monitor, draw_network(monitor, unit), draw_network(None, unit), false);
    }
}

fn draw_network(monitor: Option<Duration>, unit: Option<Unit>) {
    let networks_with_speed = network::get_networks_with_speed(match monitor {
        Some(monitor) => monitor,
        None => DEFAULT_INTERVAL,
    })
    .unwrap();

    let networks_with_speed_len = networks_with_speed.len();

    let output = get_stdout_output();
    let mut stdout = output.buffer();

    debug_assert!(networks_with_speed_len > 0);

    let mut uploads: Vec<String> = Vec::with_capacity(networks_with_speed_len);
    let mut uploads_total: Vec<String> = Vec::with_capacity(networks_with_speed_len);

    let mut downloads: Vec<String> = Vec::with_capacity(networks_with_speed_len);
    let mut downloads_total: Vec<String> = Vec::with_capacity(networks_with_speed_len);

    for (network, network_speed) in networks_with_speed.iter() {
        let upload = Byte::from_f64_with_unit(network_speed.transmit, Unit::B).unwrap();
        let upload_total = Byte::from_u64(network.stat.transmit_bytes);

        let download = Byte::from_f64_with_unit(network_speed.receive, Unit::B).unwrap();
        let download_total = Byte::from_u64(network.stat.receive_bytes);

        let (mut upload, upload_total, mut download, download_total) = match unit {
            Some(unit) => (
                format!("{:.2}", upload.get_adjusted_unit(unit)),
                format!("{:.2}", upload_total.get_adjusted_unit(unit)),
                format!("{:.2}", download.get_adjusted_unit(unit)),
                format!("{:.2}", download_total.get_adjusted_unit(unit)),
            ),
            None => (
                format!("{:.2}", upload.get_appropriate_unit(UnitType::Decimal)),
                format!("{:.2}", upload_total.get_appropriate_unit(UnitType::Decimal)),
                format!("{:.2}", download.get_appropriate_unit(UnitType::Decimal)),
                format!("{:.2}", download_total.get_appropriate_unit(UnitType::Decimal)),
            ),
        };

        upload.push_str("/s");
        download.push_str("/s");

        uploads.push(upload);
        uploads_total.push(upload_total);
        downloads.push(download);
        downloads_total.push(download_total);
    }

    let interface_len =
        networks_with_speed.iter().map(|(network, _)| network.interface.len()).max().unwrap();
    let interface_len_inc = interface_len + 1;

    let upload_len = uploads.iter().map(|upload| upload.len()).max().unwrap().max(11);
    let upload_total_len =
        uploads_total.iter().map(|upload_total| upload_total.len()).max().unwrap().max(13);
    let download_len = downloads.iter().map(|download| download.len()).max().unwrap().max(13);
    let download_total_len =
        downloads_total.iter().map(|download_total| download_total.len()).max().unwrap().max(15);

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "{1:>0$}", interface_len_inc + upload_len, "Upload Rate").unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " | ").unwrap();

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "{1:>0$}", upload_total_len, "Uploaded Data").unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " | ").unwrap();

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "{1:>0$}", download_len, "Download Rate").unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " | ").unwrap();

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "{1:>0$}", download_total_len, "Downloaded Data").unwrap();

    writeln!(&mut stdout).unwrap();

    let mut uploads_iter = uploads.into_iter();
    let mut uploads_total_iter = uploads_total.into_iter();
    let mut downloads_iter = downloads.into_iter();
    let mut downloads_total_iter = downloads_total.into_iter();

    for (network, _) in networks_with_speed.into_iter() {
        let upload = uploads_iter.next().unwrap();
        let upload_total = uploads_total_iter.next().unwrap();

        let download = downloads_iter.next().unwrap();
        let download_total = downloads_total_iter.next().unwrap();

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:<0$}", interface_len_inc, network.interface).unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

        for _ in 0..(upload_len - upload.len()) {
            write!(&mut stdout, " ").unwrap();
        }

        stdout.write_all(upload.as_bytes()).unwrap();

        write!(&mut stdout, "   ").unwrap();

        for _ in 0..(upload_total_len - upload_total.len()) {
            write!(&mut stdout, " ").unwrap();
        }

        stdout.write_all(upload_total.as_bytes()).unwrap();

        write!(&mut stdout, "   ").unwrap();

        for _ in 0..(download_len - download.len()) {
            write!(&mut stdout, " ").unwrap();
        }

        stdout.write_all(download.as_bytes()).unwrap();

        write!(&mut stdout, "   ").unwrap();

        for _ in 0..(download_total_len - download_total.len()) {
            write!(&mut stdout, " ").unwrap();
        }

        stdout.write_all(download_total.as_bytes()).unwrap();

        stdout.set_color(&COLOR_DEFAULT).unwrap();
        writeln!(&mut stdout).unwrap();
    }

    output.print(&stdout).unwrap();
}
