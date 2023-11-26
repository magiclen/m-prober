use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::{cpu, load_average};

use crate::{terminal::*, CLIArgs, CLICommands};

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

fn draw_cpu_info(monitor: Option<Duration>, separate: bool, only_information: bool) {
    let output = get_stdout_output();
    let mut stdout = output.buffer();

    let terminal_width = get_term_width();

    let mut draw_load_average = |cpus: &[cpu::CPU]| {
        let load_average = load_average::get_load_average().unwrap();

        let logical_cores_number: usize = cpus.iter().map(|cpu| cpu.siblings).sum();
        let logical_cores_number_f64 = logical_cores_number as f64;

        let one = format!("{:.2}", load_average.one);
        let five = format!("{:.2}", load_average.five);
        let fifteen = format!("{:.2}", load_average.fifteen);

        let load_average_len = one.len().max(five.len()).max(fifteen.len());

        let one_percentage =
            format!("{:.2}%", load_average.one * 100f64 / logical_cores_number_f64);
        let five_percentage =
            format!("{:.2}%", load_average.five * 100f64 / logical_cores_number_f64);
        let fifteen_percentage =
            format!("{:.2}%", load_average.fifteen * 100f64 / logical_cores_number_f64);

        let percentage_len =
            one_percentage.len().max(five_percentage.len()).max(fifteen_percentage.len());

        let progress_max = terminal_width - 11 - load_average_len - 2 - percentage_len - 1;

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

        // one

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "one    ").unwrap(); // 7

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " [").unwrap(); // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.one * f).floor() as usize).min(progress_max);

        stdout.set_color(&COLOR_USED).unwrap();
        for _ in 0..progress_used {
            write!(&mut stdout, "|").unwrap(); // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, "] ").unwrap(); // 2

        for _ in 0..(load_average_len - one.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        stdout.write_all(one.as_bytes()).unwrap();

        write!(&mut stdout, " (").unwrap(); // 2

        for _ in 0..(percentage_len - one_percentage.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.write_all(one_percentage.as_bytes()).unwrap();

        write!(&mut stdout, ")").unwrap(); // 1

        writeln!(&mut stdout).unwrap();

        // five

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "five   ").unwrap(); // 7

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " [").unwrap(); // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.five * f).floor() as usize).min(progress_max);

        stdout.set_color(&COLOR_USED).unwrap();
        for _ in 0..progress_used {
            write!(&mut stdout, "|").unwrap(); // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, "] ").unwrap(); // 2

        for _ in 0..(load_average_len - five.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        stdout.write_all(five.as_bytes()).unwrap();

        write!(&mut stdout, " (").unwrap(); // 2

        for _ in 0..(percentage_len - five_percentage.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.write_all(five_percentage.as_bytes()).unwrap();

        write!(&mut stdout, ")").unwrap(); // 1

        writeln!(&mut stdout).unwrap();

        // fifteen

        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "fifteen").unwrap(); // 7

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " [").unwrap(); // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.fifteen * f).floor() as usize).min(progress_max);

        stdout.set_color(&COLOR_USED).unwrap();
        for _ in 0..progress_used {
            write!(&mut stdout, "|").unwrap(); // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, "] ").unwrap(); // 2

        for _ in 0..(load_average_len - fifteen.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        stdout.write_all(fifteen.as_bytes()).unwrap();

        write!(&mut stdout, " (").unwrap(); // 2

        for _ in 0..(percentage_len - fifteen_percentage.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
        }

        stdout.write_all(fifteen_percentage.as_bytes()).unwrap();

        write!(&mut stdout, ")").unwrap(); // 1

        writeln!(&mut stdout).unwrap();
        writeln!(&mut stdout).unwrap();
    };

    if separate {
        let all_percentage: Vec<f64> = if only_information {
            Vec::new()
        } else {
            cpu::get_all_cpu_utilization_in_percentage(false, match monitor {
                Some(monitor) => monitor,
                None => DEFAULT_INTERVAL,
            })
            .unwrap()
        };

        let cpus = cpu::get_cpus().unwrap();

        draw_load_average(&cpus);

        let mut i = 0;

        let cpus_len_dec = cpus.len() - 1;

        for (cpu_index, cpu) in cpus.into_iter().enumerate() {
            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            stdout.write_all(cpu.model_name.as_bytes()).unwrap();

            write!(&mut stdout, " ").unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings).unwrap();

            writeln!(&mut stdout).unwrap();

            let mut hz_string: Vec<String> = Vec::with_capacity(cpu.siblings);

            for cpu_mhz in cpu.cpus_mhz.iter().copied() {
                let cpu_hz = Byte::from_f64_with_unit(cpu_mhz, Unit::MB)
                    .unwrap()
                    .get_appropriate_unit(UnitType::Decimal);

                hz_string.push(format!(
                    "{:.2} {}Hz",
                    cpu_hz.get_value(),
                    &cpu_hz.get_unit().as_str()[..1]
                ));
            }

            let hz_string_len = hz_string.iter().map(|s| s.len()).max().unwrap();

            // The max length of `CPU<number> `.
            let d = {
                let mut n = cpu.siblings;

                let mut d = 1;

                while n > 10 {
                    n /= 10;

                    d += 1;
                }

                d + 4
            };

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
                let mut percentage_string: Vec<String> = Vec::with_capacity(cpu.siblings);

                for p in all_percentage[i..].iter().copied().take(cpu.siblings) {
                    percentage_string.push(format!("{:.2}%", p * 100f64));
                }

                let percentage_len = percentage_string.iter().map(|s| s.len()).max().unwrap();

                let progress_max = terminal_width - d - 3 - percentage_len - 2 - hz_string_len - 1;

                let mut percentage_string_iter = percentage_string.into_iter();
                let mut hz_string_iter = hz_string.into_iter();

                for (i, p) in all_percentage[i..].iter().take(cpu.siblings).enumerate() {
                    let percentage_string = percentage_string_iter.next().unwrap();
                    let hz_string = hz_string_iter.next().unwrap();

                    stdout.set_color(&COLOR_LABEL).unwrap();
                    write!(&mut stdout, "{1:<0$}", d, format!("CPU{i}")).unwrap();

                    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
                    write!(&mut stdout, "[").unwrap(); // 1

                    let f = progress_max as f64;

                    let progress_used = (p * f).floor() as usize;

                    stdout.set_color(&COLOR_USED).unwrap();
                    for _ in 0..progress_used {
                        write!(&mut stdout, "|").unwrap(); // 1
                    }

                    for _ in 0..(progress_max - progress_used) {
                        write!(&mut stdout, " ").unwrap(); // 1
                    }

                    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
                    write!(&mut stdout, "] ").unwrap(); // 2

                    for _ in 0..(percentage_len - percentage_string.len()) {
                        write!(&mut stdout, " ").unwrap(); // 1
                    }

                    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
                    stdout.write_all(percentage_string.as_bytes()).unwrap();

                    write!(&mut stdout, " (").unwrap(); // 2

                    for _ in 0..(hz_string_len - hz_string.len()) {
                        write!(&mut stdout, " ").unwrap(); // 1
                    }

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
                cpu::get_average_cpu_utilization_in_percentage(match monitor {
                    Some(monitor) => monitor,
                    None => DEFAULT_INTERVAL,
                })
                .unwrap();

            let average_percentage_string = format!("{:.2}%", average_percentage * 100f64);

            (average_percentage, average_percentage_string)
        };

        let cpus = cpu::get_cpus().unwrap();

        draw_load_average(&cpus);

        for cpu in cpus {
            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            stdout.write_all(cpu.model_name.as_bytes()).unwrap();

            write!(&mut stdout, " ").unwrap();

            stdout.set_color(&COLOR_BOLD_TEXT).unwrap();

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings).unwrap();

            write!(&mut stdout, " ").unwrap();

            let cpu_mhz: f64 = cpu.cpus_mhz.iter().sum::<f64>() / cpu.cpus_mhz.len() as f64;

            let cpu_hz = Byte::from_f64_with_unit(cpu_mhz, Unit::MB)
                .unwrap()
                .get_appropriate_unit(UnitType::Decimal);

            write!(&mut stdout, "{:.2}{}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1])
                .unwrap();

            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();
        }

        if !only_information {
            let progress_max = terminal_width - 7 - average_percentage_string.len();

            stdout.set_color(&COLOR_LABEL).unwrap();
            write!(&mut stdout, "CPU").unwrap(); // 3

            stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
            write!(&mut stdout, " [").unwrap(); // 2

            let f = progress_max as f64;

            let progress_used = (average_percentage * f).floor() as usize;

            stdout.set_color(&COLOR_USED).unwrap();
            for _ in 0..progress_used {
                write!(&mut stdout, "|").unwrap(); // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ").unwrap(); // 1
            }

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
