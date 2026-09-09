use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::network;

use crate::{CLIArgs, CLICommands, terminal::*};

/// The columns of one interface, formatted, so that each of them can be measured before any of them is drawn.
struct Row {
    interface:      String,
    upload:         String,
    upload_total:   String,
    download:       String,
    download_total: String,
}

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

    let output = get_stdout_output();
    let mut stdout = output.buffer();

    debug_assert!(!networks_with_speed.is_empty());

    let rows: Vec<Row> = networks_with_speed
        .into_iter()
        .map(|(network, network_speed)| {
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

            Row {
                interface: network.interface,
                upload,
                upload_total,
                download,
                download_total,
            }
        })
        .collect();

    let interface_len = rows.iter().map(|row| row.interface.len()).max().unwrap();
    let interface_len_inc = interface_len + 1;

    // Each column is at least as wide as its own heading.
    let upload_len = rows.iter().map(|row| row.upload.len()).max().unwrap().max(11);
    let upload_total_len = rows.iter().map(|row| row.upload_total.len()).max().unwrap().max(13);
    let download_len = rows.iter().map(|row| row.download.len()).max().unwrap().max(13);
    let download_total_len = rows.iter().map(|row| row.download_total.len()).max().unwrap().max(15);

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

    for row in rows {
        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:<0$}", interface_len_inc, row.interface).unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

        write!(&mut stdout, "{1:>0$}", upload_len, row.upload).unwrap();

        write!(&mut stdout, "   ").unwrap();

        write!(&mut stdout, "{1:>0$}", upload_total_len, row.upload_total).unwrap();

        write!(&mut stdout, "   ").unwrap();

        write!(&mut stdout, "{1:>0$}", download_len, row.download).unwrap();

        write!(&mut stdout, "   ").unwrap();

        write!(&mut stdout, "{1:>0$}", download_total_len, row.download_total).unwrap();

        stdout.set_color(&COLOR_DEFAULT).unwrap();
        writeln!(&mut stdout).unwrap();
    }

    output.print(&stdout).unwrap();
}
