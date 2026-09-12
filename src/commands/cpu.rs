use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::{cpu, load_average};

use crate::{CLIArgs, CLICommands, terminal::*};

/// `/proc/cpuinfo` has no `model name` field on some architectures, e.g. on arm64.
pub const UNKNOWN_CPU_MODEL_NAME: &str = "Unknown CPU";

#[inline]
pub fn handle_cpu(args: CLIArgs) {
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
            draw_cpu_info(monitor, separate, only_information),
            draw_cpu_info(None, separate, only_information),
            only_information
        );
    }
}

/// The frequency is reported in MHz, and is worth scaling once it reaches GHz. Only the first letter of the byte unit is kept, so that `MB` reads as `MHz`.
fn scale_hz(mhz: f64) -> (f64, char) {
    let hz =
        Byte::from_f64_with_unit(mhz, Unit::MB).unwrap().get_appropriate_unit(UnitType::Decimal);

    (hz.get_value(), hz.get_unit().as_str().as_bytes()[0] as char)
}

fn draw_cpu_info(monitor: Option<Duration>, separate: bool, only_information: bool) {
    let output = get_stdout_output();
    let mut stdout = output.buffer();

    let terminal_width = get_term_width();

    let mut draw_load_average = |cpus: &[cpu::CPU]| {
        let load_average = load_average::get_load_average().unwrap();

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
    };

    if separate {
        let all_percentage: Vec<f64> = if only_information {
            Vec::new()
        } else {
            cpu::get_all_cpu_utilization_in_percentage(false, monitor.unwrap_or(DEFAULT_INTERVAL))
                .unwrap()
        };

        let cpus = cpu::get_cpus().unwrap();

        draw_load_average(&cpus);

        let mut i = 0;

        let cpus_len_dec = cpus.len().saturating_sub(1);

        for (cpu_index, cpu) in cpus.into_iter().enumerate() {
            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            stdout
                .write_all(cpu.model_name.as_deref().unwrap_or(UNKNOWN_CPU_MODEL_NAME).as_bytes())
                .unwrap();

            write!(&mut stdout, " ").unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings).unwrap();

            writeln!(&mut stdout).unwrap();

            // A kernel which reports no `cpu MHz` and has no cpufreq files leaves this empty, which only means the column is not drawn.
            let hz_string: Vec<String> = cpu
                .cpus_mhz
                .iter()
                .copied()
                .map(|cpu_mhz| {
                    let (value, unit) = scale_hz(cpu_mhz);

                    format!("{value:.2} {unit}Hz")
                })
                .collect();

            let hz_string_len = hz_string.iter().map(|s| s.len()).max().unwrap_or(0);

            // The max length of `CPU<number> `, where the highest number is one below the count.
            let d = cpu.siblings.saturating_sub(1).checked_ilog10().unwrap_or(0) as usize + 5;

            if only_information {
                for (i, hz_string) in hz_string.into_iter().enumerate() {
                    stdout.set_color(&COLOR_LABEL).unwrap();
                    write!(&mut stdout, "{1:<0$}", d, format!("CPU{i}")).unwrap();

                    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
                    write!(&mut stdout, "{1:>0$}", hz_string_len, hz_string).unwrap();

                    stdout.set_color(&COLOR_DEFAULT).unwrap();
                    writeln!(&mut stdout).unwrap();
                }
            } else {
                let percentage_string: Vec<String> = all_percentage[i..]
                    .iter()
                    .copied()
                    .take(cpu.siblings)
                    .map(|p| format!("{:.2}%", p * 100f64))
                    .collect();

                let percentage_len = percentage_string.iter().map(|s| s.len()).max().unwrap_or(0);

                let progress_max =
                    bar_width(terminal_width, d + 3 + percentage_len + 2 + hz_string_len + 1);

                let mut percentage_string_iter = percentage_string.into_iter();
                let mut hz_string_iter = hz_string.into_iter();

                for (i, p) in all_percentage[i..].iter().take(cpu.siblings).enumerate() {
                    let percentage_string = percentage_string_iter.next().unwrap();
                    // The frequencies are absent on a kernel which reports none, so this column collapses to nothing.
                    let hz_string = hz_string_iter.next().unwrap_or_default();

                    stdout.set_color(&COLOR_LABEL).unwrap();
                    write!(&mut stdout, "{1:<0$}", d, format!("CPU{i}")).unwrap();

                    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
                    write!(&mut stdout, "[").unwrap(); // 1

                    let progress_used = (p * progress_max as f64).floor() as usize;

                    stdout.set_color(&COLOR_USED).unwrap();
                    write_cells(&mut stdout, b'|', progress_used).unwrap();

                    write_cells(&mut stdout, b' ', progress_max - progress_used).unwrap();

                    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
                    write!(&mut stdout, "] ").unwrap(); // 2

                    write!(&mut stdout, "{:1$}", "", percentage_len - percentage_string.len())
                        .unwrap();

                    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
                    stdout.write_all(percentage_string.as_bytes()).unwrap();

                    write!(&mut stdout, " (").unwrap(); // 2

                    write!(&mut stdout, "{:1$}", "", hz_string_len - hz_string.len()).unwrap();

                    stdout.write_all(hz_string.as_bytes()).unwrap();

                    write!(&mut stdout, ")").unwrap(); // 1

                    stdout.set_color(&COLOR_DEFAULT).unwrap();
                    writeln!(&mut stdout).unwrap();
                }

                i += cpu.siblings;
            }

            if cpu_index != cpus_len_dec {
                writeln!(&mut stdout).unwrap();
            }
        }
    } else {
        let (average_percentage, average_percentage_string) = if only_information {
            (0f64, "".to_string())
        } else {
            let average_percentage =
                cpu::get_average_cpu_utilization_in_percentage(monitor.unwrap_or(DEFAULT_INTERVAL))
                    .unwrap();

            let average_percentage_string = format!("{:.2}%", average_percentage * 100f64);

            (average_percentage, average_percentage_string)
        };

        let cpus = cpu::get_cpus().unwrap();

        draw_load_average(&cpus);

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

                write!(&mut stdout, "{value:.2}{unit}Hz").unwrap();
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

            let progress_used = (average_percentage * progress_max as f64).floor() as usize;

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

    output.print(&stdout).unwrap();
}
