use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::volume;

use crate::{CLIArgs, CLICommands, terminal::*};

/// The columns above the usage bar when only the totals are shown.
const INFORMATION_HEADERS: [&str; 2] = ["Read Data", "Written Data"];

/// The columns above the usage bar when the I/O rates are measured too.
const SPEED_HEADERS: [&str; 4] = ["Reading Rate", "Read Data", "Writing Rate", "Written Data"];

/// One volume, formatted. `columns` holds the I/O figures, which are the only part that differs between the two modes; everything below them is the same either way.
struct Row {
    device:          String,
    columns:         Vec<String>,
    used:            u64,
    size:            u64,
    used_text:       String,
    size_text:       String,
    used_percentage: String,
    points:          Vec<String>,
}

impl Row {
    fn new(
        volume: volume::Volume,
        columns: Vec<String>,
        format_byte: impl Fn(Byte) -> String,
    ) -> Self {
        Row {
            used: volume.used,
            size: volume.size,
            used_text: format_byte(Byte::from_u64(volume.used)),
            size_text: format_byte(Byte::from_u64(volume.size)),
            used_percentage: format!("{:.2}%", percentage_of(volume.used, volume.size)),
            device: volume.device,
            points: volume.points,
            columns,
        }
    }
}

#[inline]
pub fn handle_volume(args: CLIArgs) {
    debug_assert!(matches!(args.command, CLICommands::Volume { .. }));

    if let CLICommands::Volume {
        plain,
        light,
        monitor,
        unit,
        only_information,
        mounts,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(
            monitor,
            draw_volume(monitor, unit, only_information, mounts),
            draw_volume(None, unit, only_information, mounts),
            only_information
        );
    }
}

fn draw_volume(
    monitor: Option<Duration>,
    unit: Option<Unit>,
    only_information: bool,
    mounts: bool,
) {
    let output = get_stdout_output();
    let mut stdout = output.buffer();

    let terminal_width = get_term_width();

    let format_byte = |byte: Byte| match unit {
        Some(unit) => format!("{:.2}", byte.get_adjusted_unit(unit)),
        None => format!("{:.2}", byte.get_appropriate_unit(UnitType::Decimal)),
    };

    let format_rate = |bytes_per_second: f64| {
        let mut rate = format_byte(Byte::from_f64_with_unit(bytes_per_second, Unit::B).unwrap());

        rate.push_str("/s");

        rate
    };

    let (headers, rows): (&[&str], Vec<Row>) = if only_information {
        let volumes = volume::get_volumes().unwrap();

        let rows = volumes
            .into_iter()
            .map(|volume| {
                let columns = vec![
                    format_byte(Byte::from_u64(volume.stat.read_bytes)),
                    format_byte(Byte::from_u64(volume.stat.write_bytes)),
                ];

                Row::new(volume, columns, format_byte)
            })
            .collect();

        (&INFORMATION_HEADERS, rows)
    } else {
        let volumes_with_speed =
            volume::get_volumes_with_speed(monitor.unwrap_or(DEFAULT_INTERVAL)).unwrap();

        let rows = volumes_with_speed
            .into_iter()
            .map(|(volume, volume_speed)| {
                let columns = vec![
                    format_rate(volume_speed.read),
                    format_byte(Byte::from_u64(volume.stat.read_bytes)),
                    format_rate(volume_speed.write),
                    format_byte(Byte::from_u64(volume.stat.write_bytes)),
                ];

                Row::new(volume, columns, format_byte)
            })
            .collect();

        (&SPEED_HEADERS, rows)
    };

    if rows.is_empty() {
        // A container whose mounts are all overlay or tmpfs has no device in `/proc/diskstats` to match.
        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        writeln!(&mut stdout, "There are no volumes to show.").unwrap();

        stdout.set_color(&COLOR_DEFAULT).unwrap();
    } else {
        draw_rows(&mut stdout, headers, &rows, terminal_width, mounts);
    }

    output.print(&stdout).unwrap();
}

fn draw_rows(
    stdout: &mut impl WriteColor,
    headers: &[&str],
    rows: &[Row],
    terminal_width: usize,
    mounts: bool,
) {
    let devices_len = rows.iter().map(|row| display_width(&row.device)).max().unwrap();
    let devices_len_inc = devices_len + 1;

    let used_len = rows.iter().map(|row| row.used_text.len()).max().unwrap();
    let size_len = rows.iter().map(|row| row.size_text.len()).max().unwrap();
    let percentage_len = rows.iter().map(|row| row.used_percentage.len()).max().unwrap();

    // Each column is at least as wide as its own heading.
    let column_len: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter().map(|row| row.columns[index].len()).max().unwrap().max(header.len())
        })
        .collect();

    let progress_max = bar_width(
        terminal_width,
        devices_len + 4 + used_len + 3 + size_len + 2 + percentage_len + 1,
    );

    // The first heading is right-aligned over the device column as well, since nothing is printed above the device names.
    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(stdout, "{1:>0$}", devices_len_inc + column_len[0], headers[0]).unwrap();

    for (header, width) in headers.iter().zip(column_len.iter()).skip(1) {
        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(stdout, " | ").unwrap();

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(stdout, "{1:>0$}", width, header).unwrap();
    }

    writeln!(stdout).unwrap();

    for row in rows {
        stdout.set_color(&COLOR_LABEL).unwrap();
        write_left_aligned(stdout, &row.device, devices_len_inc).unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

        for (index, (column, width)) in row.columns.iter().zip(column_len.iter()).enumerate() {
            if index > 0 {
                write!(stdout, "   ").unwrap();
            }

            write!(stdout, "{1:>0$}", width, column).unwrap();
        }

        writeln!(stdout).unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(stdout, "{:1$}", "", devices_len).unwrap();

        write!(stdout, " [").unwrap(); // 2

        let mut remaining = progress_max;

        let progress_used = bar_cells(row.used, row.size, progress_max, &mut remaining);

        stdout.set_color(&COLOR_USED).unwrap();
        write_cells(stdout, b'|', progress_used).unwrap();

        write_cells(stdout, b' ', remaining).unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(stdout, "] ").unwrap(); // 2

        write!(stdout, "{:1$}", "", used_len - row.used_text.len()).unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        stdout.write_all(row.used_text.as_bytes()).unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(stdout, " / ").unwrap(); // 3

        write!(stdout, "{:1$}", "", size_len - row.size_text.len()).unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        stdout.write_all(row.size_text.as_bytes()).unwrap();

        write!(stdout, " (").unwrap(); // 2

        write!(stdout, "{:1$}", "", percentage_len - row.used_percentage.len()).unwrap();

        stdout.write_all(row.used_percentage.as_bytes()).unwrap();

        write!(stdout, ")").unwrap(); // 1

        stdout.set_color(&COLOR_DEFAULT).unwrap();
        writeln!(stdout).unwrap();

        if mounts {
            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();

            for point in row.points.iter() {
                write!(stdout, "{:1$}", "", devices_len_inc).unwrap();

                stdout.write_all(point.as_bytes()).unwrap();

                stdout.set_color(&COLOR_DEFAULT).unwrap();
                writeln!(stdout).unwrap();
            }
        }
    }
}
