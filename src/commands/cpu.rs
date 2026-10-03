use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::{cpu, load_average};

use crate::{
    CLIArgs, CLICommands,
    cpu_sample::{self, CpuThreadSnapshot},
    terminal::*,
};

/// `/proc/cpuinfo` has no `model name` field on some architectures, e.g. on arm64.
pub const UNKNOWN_CPU_MODEL_NAME: &str = "Unknown CPU";

#[inline]
pub fn handle_cpu(args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(args.command, CLICommands::Cpu { .. }));

    if let CLICommands::Cpu {
        plain,
        light,
        monitor,
        separate,
        only_information,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(
            monitor,
            draw_cpu_info(monitor, separate, only_information)?,
            draw_cpu_info(None, separate, only_information)?,
            only_information
        );
    }

    Ok(())
}

/// The frequency is reported in MHz, and is worth scaling once it reaches GHz. Only the first letter of the byte unit is kept, so that `MB` reads as `MHz`.
fn scale_hz(mhz: f64) -> (f64, char) {
    let hz =
        Byte::from_f64_with_unit(mhz, Unit::MB).unwrap().get_appropriate_unit(UnitType::Decimal);

    (hz.get_value(), hz.get_unit().as_str().as_bytes()[0] as char)
}

/// Split the CPUs by the package they belong to, in the order of `package_ids`, and hand back the ones of an unknown package on their own.
///
/// Many machines with more than one socket number the CPUs of the packages in turn rather than one package after another, e.g. `0-13,28-41` for the first one, so a CPU is matched by its package rather than by its position.
fn group_by_package<'a>(
    package_ids: &[usize],
    threads: &'a [CpuThreadSnapshot],
) -> (Vec<Vec<&'a CpuThreadSnapshot>>, Vec<&'a CpuThreadSnapshot>) {
    let mut groups = vec![Vec::new(); package_ids.len()];
    let mut unknown = Vec::new();

    for thread in threads {
        match thread
            .physical_id
            .and_then(|physical_id| package_ids.iter().position(|id| *id == physical_id))
        {
            Some(index) => groups[index].push(thread),
            None => unknown.push(thread),
        }
    }

    (groups, unknown)
}

/// Draw one row per CPU, labelled with the number the kernel gives it.
fn draw_threads(
    stdout: &mut impl WriteColor,
    threads: &[&CpuThreadSnapshot],
    only_information: bool,
    terminal_width: usize,
) {
    let label_len = threads.iter().map(|thread| thread.id.to_string().len()).max().unwrap_or(0) + 4;

    let hz_string: Vec<String> = threads
        .iter()
        .map(|thread| match thread.frequency_mhz {
            Some(cpu_mhz) => {
                let (value, unit) = scale_hz(cpu_mhz);

                format!("{value:.2} {unit}Hz")
            },
            None => String::new(),
        })
        .collect();

    let hz_string_len = hz_string.iter().map(|s| s.len()).max().unwrap_or(0);

    if only_information {
        for (thread, hz_string) in threads.iter().zip(hz_string) {
            // There is nothing else to show about a CPU without a frequency.
            if hz_string.is_empty() {
                continue;
            }

            stdout.set_color(&COLOR_LABEL).unwrap();
            write!(stdout, "{1:<0$}", label_len, format!("CPU{}", thread.id)).unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            write!(stdout, "{1:>0$}", hz_string_len, hz_string).unwrap();

            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(stdout).unwrap();
        }

        return;
    }

    // A CPU which came online during the interval has no usage yet.
    let percentage_string: Vec<String> = threads
        .iter()
        .map(|thread| match thread.usage {
            Some(usage) => format!("{:.2}%", usage * 100f64),
            None => String::from("N/A"),
        })
        .collect();

    let percentage_len = percentage_string.iter().map(|s| s.len()).max().unwrap_or(0);

    // The frequencies are absent on a kernel which reports none, so their column collapses to nothing.
    let hz_column_len = if hz_string_len > 0 { 2 + hz_string_len + 1 } else { 0 };

    let progress_max = bar_width(terminal_width, label_len + 3 + percentage_len + hz_column_len);

    for ((thread, percentage_string), hz_string) in
        threads.iter().zip(percentage_string).zip(hz_string)
    {
        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(stdout, "{1:<0$}", label_len, format!("CPU{}", thread.id)).unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(stdout, "[").unwrap(); // 1

        // `/proc/stat` can hand back a ratio above one when its counters are reset, e.g. by a CPU going offline, and the blanks below would then underflow.
        let progress_used = ((thread.usage.unwrap_or(0f64) * progress_max as f64).floor() as usize)
            .min(progress_max);

        stdout.set_color(&COLOR_USED).unwrap();
        write_cells(stdout, b'|', progress_used).unwrap();

        write_cells(stdout, b' ', progress_max - progress_used).unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(stdout, "] ").unwrap(); // 2

        write!(stdout, "{:1$}", "", percentage_len - percentage_string.len()).unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        stdout.write_all(percentage_string.as_bytes()).unwrap();

        if hz_string_len > 0 {
            write!(stdout, " (").unwrap(); // 2

            write!(stdout, "{:1$}", "", hz_string_len - hz_string.len()).unwrap();

            stdout.write_all(hz_string.as_bytes()).unwrap();

            write!(stdout, ")").unwrap(); // 1
        }

        stdout.set_color(&COLOR_DEFAULT).unwrap();
        writeln!(stdout).unwrap();
    }
}

fn draw_cpu_info(
    monitor: Option<Duration>,
    separate: bool,
    only_information: bool,
) -> anyhow::Result<()> {
    let output = get_stdout_output();
    let mut stdout = output.buffer();

    let terminal_width = get_term_width();

    let mut draw_load_average = |cpus: &[cpu::CPU]| -> anyhow::Result<()> {
        let load_average = load_average::get_load_average()?;

        let logical_cores_number: usize = cpus.iter().map(|cpu| cpu.siblings).sum();
        let logical_cores_number_f64 = logical_cores_number as f64;

        let rows = [
            ("one    ", load_average.one),
            ("five   ", load_average.five),
            ("fifteen", load_average.fifteen),
        ]
        .map(|(label, value)| {
            let value_string = format!("{value:.2}");
            let percentage_string = format!("{:.2}%", value * 100f64 / logical_cores_number_f64);

            (label, value, value_string, percentage_string)
        });

        let load_average_len = rows.iter().map(|(_, _, value, _)| value.len()).max().unwrap();
        let percentage_len =
            rows.iter().map(|(_, _, _, percentage)| percentage.len()).max().unwrap();

        let progress_max =
            bar_width(terminal_width, 11 + load_average_len + 2 + percentage_len + 1);

        // number of logical CPU cores

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        if logical_cores_number > 1 {
            write!(&mut stdout, "There are ").unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            write!(&mut stdout, "{logical_cores_number}").unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, " logical CPU cores.").unwrap();
        } else {
            write!(&mut stdout, "There is only one logical CPU core.").unwrap();
        }
        writeln!(&mut stdout).unwrap();

        let f = progress_max as f64 / logical_cores_number_f64;

        for (label, value, value_string, percentage_string) in rows {
            stdout.set_color(&COLOR_LABEL).unwrap();
            write!(&mut stdout, "{label}").unwrap(); // 7

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, " [").unwrap(); // 2

            let progress_used = ((value * f).floor() as usize).min(progress_max);

            stdout.set_color(&COLOR_USED).unwrap();
            write_cells(&mut stdout, b'|', progress_used).unwrap();

            write_cells(&mut stdout, b' ', progress_max - progress_used).unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, "] ").unwrap(); // 2

            write!(&mut stdout, "{:1$}", "", load_average_len - value_string.len()).unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            stdout.write_all(value_string.as_bytes()).unwrap();

            write!(&mut stdout, " (").unwrap(); // 2

            write!(&mut stdout, "{:1$}", "", percentage_len - percentage_string.len()).unwrap();

            stdout.write_all(percentage_string.as_bytes()).unwrap();

            write!(&mut stdout, ")").unwrap(); // 1

            writeln!(&mut stdout).unwrap();
        }

        writeln!(&mut stdout).unwrap();

        Ok(())
    };

    if separate {
        let threads = if only_information {
            cpu_sample::read_threads()?
        } else {
            cpu_sample::sample(monitor.unwrap_or(DEFAULT_INTERVAL))?.threads
        };

        let cpus = cpu::get_cpus()?;

        draw_load_average(&cpus)?;

        let package_ids: Vec<usize> = cpus.iter().map(|cpu| cpu.physical_id).collect();

        let (groups, unknown) = group_by_package(&package_ids, &threads);

        for (index, cpu) in cpus.iter().enumerate() {
            if index > 0 {
                writeln!(&mut stdout).unwrap();
            }

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            stdout
                .write_all(cpu.model_name.as_deref().unwrap_or(UNKNOWN_CPU_MODEL_NAME).as_bytes())
                .unwrap();

            write!(&mut stdout, " ").unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings).unwrap();

            writeln!(&mut stdout).unwrap();

            draw_threads(&mut stdout, &groups[index], only_information, terminal_width);
        }

        // A CPU whose package neither `/proc/cpuinfo` nor sysfs reports is still shown, apart from the others.
        if !unknown.is_empty() {
            writeln!(&mut stdout).unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            writeln!(&mut stdout, "{UNKNOWN_CPU_MODEL_NAME}").unwrap();

            draw_threads(&mut stdout, &unknown, only_information, terminal_width);
        }
    } else {
        let (average_percentage, average_percentage_string) = if only_information {
            (0f64, "".to_string())
        } else {
            let average_percentage = cpu::get_average_cpu_utilization_in_percentage(
                monitor.unwrap_or(DEFAULT_INTERVAL),
            )?;

            let average_percentage_string = format!("{:.2}%", average_percentage * 100f64);

            (average_percentage, average_percentage_string)
        };

        let cpus = cpu::get_cpus()?;

        draw_load_average(&cpus)?;

        for cpu in cpus {
            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            stdout
                .write_all(cpu.model_name.as_deref().unwrap_or(UNKNOWN_CPU_MODEL_NAME).as_bytes())
                .unwrap();

            write!(&mut stdout, " ").unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings).unwrap();

            // A kernel which reports no `cpu MHz` and has no cpufreq files leaves this empty, and an average of nothing is not a number.
            if !cpu.cpus_mhz.is_empty() {
                write!(&mut stdout, " ").unwrap();

                let cpu_mhz: f64 = cpu.cpus_mhz.iter().sum::<f64>() / cpu.cpus_mhz.len() as f64;

                let (value, unit) = scale_hz(cpu_mhz);

                write!(&mut stdout, "{value:.2} {unit}Hz").unwrap();
            }

            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();
        }

        if !only_information {
            let progress_max = bar_width(terminal_width, 7 + average_percentage_string.len());

            stdout.set_color(&COLOR_LABEL).unwrap();
            write!(&mut stdout, "CPU").unwrap(); // 3

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, " [").unwrap(); // 2

            let progress_used =
                ((average_percentage * progress_max as f64).floor() as usize).min(progress_max);

            stdout.set_color(&COLOR_USED).unwrap();
            write_cells(&mut stdout, b'|', progress_used).unwrap();

            write_cells(&mut stdout, b' ', progress_max - progress_used).unwrap();

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, "] ").unwrap(); // 2

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
            stdout.write_all(average_percentage_string.as_bytes()).unwrap();

            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();
        }
    }

    output.print(&stdout)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_package_of_interleaved_cpu_numbers() {
        let thread = |id, physical_id| CpuThreadSnapshot {
            id,
            physical_id,
            usage: None,
            frequency_mhz: None,
        };

        // Two sockets whose CPUs take turns, one with no package at all, and one of a package `/proc/cpuinfo` does not list.
        let threads = [
            thread(0, Some(0)),
            thread(1, Some(1)),
            thread(2, Some(0)),
            thread(3, Some(1)),
            thread(4, None),
            thread(5, Some(7)),
        ];

        let (groups, unknown) = group_by_package(&[0, 1], &threads);

        let ids =
            |group: &[&CpuThreadSnapshot]| group.iter().map(|thread| thread.id).collect::<Vec<_>>();

        assert_eq!(vec![0, 2], ids(&groups[0]));
        assert_eq!(vec![1, 3], ids(&groups[1]));
        assert_eq!(vec![4, 5], ids(&unknown));
    }
}
