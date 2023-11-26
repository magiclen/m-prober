use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::volume;

use crate::{terminal::*, CLIArgs, CLICommands};

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

    if only_information {
        let volumes = volume::get_volumes().unwrap();

        let volumes_len = volumes.len();

        debug_assert!(volumes_len > 0);

        let mut volumes_size: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_used: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_used_percentage: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_read_total: Vec<String> = Vec::with_capacity(volumes_len);

        let mut volumes_write_total: Vec<String> = Vec::with_capacity(volumes_len);

        for volume in volumes.iter() {
            let size = Byte::from_u64(volume.size);

            let used = Byte::from_u64(volume.used);

            let used_percentage =
                format!("{:.2}%", (volume.used * 100) as f64 / volume.size as f64);

            let read_total = Byte::from_u64(volume.stat.read_bytes);

            let write_total = Byte::from_u64(volume.stat.write_bytes);

            let (size, used, read_total, write_total) = match unit {
                Some(unit) => (
                    format!("{:.2}", size.get_adjusted_unit(unit)),
                    format!("{:.2}", used.get_adjusted_unit(unit)),
                    format!("{:.2}", read_total.get_adjusted_unit(unit)),
                    format!("{:.2}", write_total.get_adjusted_unit(unit)),
                ),
                None => (
                    format!("{:.2}", size.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", used.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", read_total.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", write_total.get_appropriate_unit(UnitType::Decimal)),
                ),
            };

            volumes_size.push(size);
            volumes_used.push(used);
            volumes_used_percentage.push(used_percentage);
            volumes_read_total.push(read_total);
            volumes_write_total.push(write_total);
        }

        let devices_len = volumes.iter().map(|volume| volume.device.len()).max().unwrap();
        let devices_len_inc = devices_len + 1;

        let volumes_size_len = volumes_size.iter().map(|size| size.len()).max().unwrap();
        let volumes_used_len = volumes_used.iter().map(|used| used.len()).max().unwrap();
        let volumes_used_percentage_len = volumes_used_percentage
            .iter()
            .map(|used_percentage| used_percentage.len())
            .max()
            .unwrap();
        let volumes_read_total_len =
            volumes_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let volumes_write_total_len =
            volumes_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width
            - devices_len
            - 4
            - volumes_used_len
            - 3
            - volumes_size_len
            - 2
            - volumes_used_percentage_len
            - 1;

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:>0$}", devices_len_inc + volumes_read_total_len, "Read Data")
            .unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " | ").unwrap();

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:>0$}", volumes_write_total_len, "Written Data").unwrap();

        writeln!(&mut stdout).unwrap();

        let mut volumes_size_iter = volumes_size.into_iter();
        let mut volumes_used_iter = volumes_used.into_iter();
        let mut volumes_used_percentage_iter = volumes_used_percentage.into_iter();
        let mut volumes_read_total_iter = volumes_read_total.into_iter();
        let mut volumes_write_total_iter = volumes_write_total.into_iter();

        for volume in volumes.into_iter() {
            let size = volumes_size_iter.next().unwrap();

            let used = volumes_used_iter.next().unwrap();

            let used_percentage = volumes_used_percentage_iter.next().unwrap();

            let read_total = volumes_read_total_iter.next().unwrap();

            let write_total = volumes_write_total_iter.next().unwrap();

            stdout.set_color(&COLOR_LABEL).unwrap();
            write!(&mut stdout, "{1:<0$}", devices_len_inc, volume.device).unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

            for _ in 0..(volumes_read_total_len - read_total.len()) {
                write!(&mut stdout, " ").unwrap();
            }

            stdout.write_all(read_total.as_bytes()).unwrap();

            write!(&mut stdout, "   ").unwrap();

            for _ in 0..(volumes_write_total_len - write_total.len()) {
                write!(&mut stdout, " ").unwrap();
            }

            stdout.write_all(write_total.as_bytes()).unwrap();

            writeln!(&mut stdout).unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();

            for _ in 0..devices_len {
                write!(&mut stdout, " ").unwrap();
            }

            write!(&mut stdout, " [").unwrap(); // 2

            let f = progress_max as f64 / volume.size as f64;

            let progress_used = (volume.used as f64 * f).floor() as usize;

            stdout.set_color(&COLOR_USED).unwrap();
            for _ in 0..progress_used {
                write!(&mut stdout, "|").unwrap(); // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, "] ").unwrap(); // 2

            for _ in 0..(volumes_used_len - used.len()) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            stdout.write_all(used.as_bytes()).unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, " / ").unwrap(); // 3

            for _ in 0..(volumes_size_len - size.len()) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            stdout.write_all(size.as_bytes()).unwrap();

            write!(&mut stdout, " (").unwrap(); // 2

            for _ in 0..(volumes_used_percentage_len - used_percentage.len()) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.write_all(used_percentage.as_bytes()).unwrap();

            write!(&mut stdout, ")").unwrap(); // 1

            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            if mounts {
                stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();

                for point in volume.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ").unwrap();
                    }

                    stdout.write_all(point.as_bytes()).unwrap();

                    stdout.set_color(&COLOR_DEFAULT).unwrap();
                    writeln!(&mut stdout).unwrap();
                }
            }
        }
    } else {
        let volumes_with_speed = volume::get_volumes_with_speed(match monitor {
            Some(monitor) => monitor,
            None => DEFAULT_INTERVAL,
        })
        .unwrap();

        let volumes_with_speed_len = volumes_with_speed.len();

        debug_assert!(volumes_with_speed_len > 0);

        let mut volumes_size: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_used: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_used_percentage: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_read: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_read_total: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_write: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        let mut volumes_write_total: Vec<String> = Vec::with_capacity(volumes_with_speed_len);

        for (volume, volume_speed) in volumes_with_speed.iter() {
            let size = Byte::from_u64(volume.size);

            let used = Byte::from_u64(volume.used);

            let used_percentage =
                format!("{:.2}%", (volume.used * 100) as f64 / volume.size as f64);

            let read = Byte::from_f64_with_unit(volume_speed.read, Unit::B).unwrap();
            let read_total = Byte::from_u64(volume.stat.read_bytes);

            let write = Byte::from_f64_with_unit(volume_speed.write, Unit::B).unwrap();
            let write_total = Byte::from_u64(volume.stat.read_bytes);

            let (size, used, mut read, read_total, mut write, write_total) = match unit {
                Some(unit) => (
                    format!("{:.2}", size.get_adjusted_unit(unit)),
                    format!("{:.2}", used.get_adjusted_unit(unit)),
                    format!("{:.2}", read.get_adjusted_unit(unit)),
                    format!("{:.2}", read_total.get_adjusted_unit(unit)),
                    format!("{:.2}", write.get_adjusted_unit(unit)),
                    format!("{:.2}", write_total.get_adjusted_unit(unit)),
                ),
                None => (
                    format!("{:.2}", size.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", used.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", read.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", read_total.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", write.get_appropriate_unit(UnitType::Decimal)),
                    format!("{:.2}", write_total.get_appropriate_unit(UnitType::Decimal)),
                ),
            };

            read.push_str("/s");
            write.push_str("/s");

            volumes_size.push(size);
            volumes_used.push(used);
            volumes_used_percentage.push(used_percentage);
            volumes_read.push(read);
            volumes_read_total.push(read_total);
            volumes_write.push(write);
            volumes_write_total.push(write_total);
        }

        let devices_len =
            volumes_with_speed.iter().map(|(volume, _)| volume.device.len()).max().unwrap();
        let devices_len_inc = devices_len + 1;

        let volumes_size_len = volumes_size.iter().map(|size| size.len()).max().unwrap();
        let volumes_used_len = volumes_used.iter().map(|used| used.len()).max().unwrap();
        let volumes_used_percentage_len = volumes_used_percentage
            .iter()
            .map(|used_percentage| used_percentage.len())
            .max()
            .unwrap();
        let volumes_read_len = volumes_read.iter().map(|read| read.len()).max().unwrap().max(12);
        let volumes_read_total_len =
            volumes_read_total.iter().map(|read_total| read_total.len()).max().unwrap().max(9);
        let volumes_write_len =
            volumes_write.iter().map(|write| write.len()).max().unwrap().max(12);
        let volumes_write_total_len =
            volumes_write_total.iter().map(|write_total| write_total.len()).max().unwrap().max(12);

        let progress_max = terminal_width
            - devices_len
            - 4
            - volumes_used_len
            - 3
            - volumes_size_len
            - 2
            - volumes_used_percentage_len
            - 1;

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:>0$}", devices_len_inc + volumes_read_len, "Reading Rate").unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " | ").unwrap();

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:>0$}", volumes_read_total_len, "Read Data").unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " | ").unwrap();

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:>0$}", volumes_write_len, "Writing Rate").unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " | ").unwrap();

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{1:>0$}", volumes_write_total_len, "Written Data").unwrap();

        writeln!(&mut stdout).unwrap();

        let mut volumes_size_iter = volumes_size.into_iter();
        let mut volumes_used_iter = volumes_used.into_iter();
        let mut volumes_used_percentage_iter = volumes_used_percentage.into_iter();
        let mut volumes_read_iter = volumes_read.into_iter();
        let mut volumes_read_total_iter = volumes_read_total.into_iter();
        let mut volumes_write_iter = volumes_write.into_iter();
        let mut volumes_write_total_iter = volumes_write_total.into_iter();

        for (volume, _) in volumes_with_speed.into_iter() {
            let size = volumes_size_iter.next().unwrap();

            let used = volumes_used_iter.next().unwrap();

            let used_percentage = volumes_used_percentage_iter.next().unwrap();

            let read = volumes_read_iter.next().unwrap();
            let read_total = volumes_read_total_iter.next().unwrap();

            let write = volumes_write_iter.next().unwrap();
            let write_total = volumes_write_total_iter.next().unwrap();

            stdout.set_color(&COLOR_LABEL).unwrap();
            write!(&mut stdout, "{1:<0$}", devices_len_inc, volume.device).unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

            for _ in 0..(volumes_read_len - read.len()) {
                write!(&mut stdout, " ").unwrap();
            }

            stdout.write_all(read.as_bytes()).unwrap();

            write!(&mut stdout, "   ").unwrap();

            for _ in 0..(volumes_read_total_len - read_total.len()) {
                write!(&mut stdout, " ").unwrap();
            }

            stdout.write_all(read_total.as_bytes()).unwrap();

            write!(&mut stdout, "   ").unwrap();

            for _ in 0..(volumes_write_len - write.len()) {
                write!(&mut stdout, " ").unwrap();
            }

            stdout.write_all(write.as_bytes()).unwrap();

            write!(&mut stdout, "   ").unwrap();

            for _ in 0..(volumes_write_total_len - write_total.len()) {
                write!(&mut stdout, " ").unwrap();
            }

            stdout.write_all(write_total.as_bytes()).unwrap();

            writeln!(&mut stdout).unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();

            for _ in 0..devices_len {
                write!(&mut stdout, " ").unwrap();
            }

            write!(&mut stdout, " [").unwrap(); // 2

            let f = progress_max as f64 / volume.size as f64;

            let progress_used = (volume.used as f64 * f).floor() as usize;

            stdout.set_color(&COLOR_USED).unwrap();
            for _ in 0..progress_used {
                write!(&mut stdout, "|").unwrap(); // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, "] ").unwrap(); // 2

            for _ in 0..(volumes_used_len - used.len()) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            stdout.write_all(used.as_bytes()).unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, " / ").unwrap(); // 3

            for _ in 0..(volumes_size_len - size.len()) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            stdout.write_all(size.as_bytes()).unwrap();

            write!(&mut stdout, " (").unwrap(); // 2

            for _ in 0..(volumes_used_percentage_len - used_percentage.len()) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

            stdout.write_all(used_percentage.as_bytes()).unwrap();

            write!(&mut stdout, ")").unwrap(); // 1

            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            if mounts {
                stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();

                for point in volume.points {
                    for _ in 0..devices_len_inc {
                        write!(&mut stdout, " ").unwrap();
                    }

                    stdout.write_all(point.as_bytes()).unwrap();

                    stdout.set_color(&COLOR_DEFAULT).unwrap();
                    writeln!(&mut stdout).unwrap();
                }
            }
        }
    }

    output.print(&stdout).unwrap();
}
