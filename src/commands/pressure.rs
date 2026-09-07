use anyhow::anyhow;
use mprober_lib::pressure::{self, PressureStat};

use crate::{CLIArgs, CLICommands, terminal::*};

/// The width of `Memory some`, which is the longest label.
const LABEL_WIDTH: usize = 11;

/// Three columns of ` 100.00%`.
const VALUES_WIDTH: usize = 24;

#[inline]
pub fn handle_pressure(args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(args.command, CLICommands::Pressure { .. }));

    if let CLICommands::Pressure {
        plain,
        light,
        monitor,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(monitor, draw_pressure()?);
    }

    Ok(())
}

fn draw_pressure() -> anyhow::Result<()> {
    let cpu = match pressure::get_cpu_pressure() {
        Ok(cpu) => cpu,
        Err(error) if error.is_not_supported() => {
            return Err(anyhow!(
                "This kernel does not provide PSI. It needs `CONFIG_PSI` and must not be booted \
                 with `psi=0`."
            ));
        },
        Err(error) => return Err(error.into()),
    };

    let memory = pressure::get_memory_pressure()?;
    let io = pressure::get_io_pressure()?;

    let mut rows: Vec<(&str, &str, &PressureStat)> = Vec::with_capacity(6);

    for (name, pressure) in [("CPU", &cpu), ("Memory", &memory), ("IO", &io)] {
        rows.push((name, "some", &pressure.some));

        // The CPU has no `full` line on kernels older than 5.13, where a fully stalled CPU was not accounted for.
        if let Some(full) = pressure.full.as_ref() {
            rows.push((name, "full", full));
        }
    }

    let output = get_stdout_output();
    let mut stdout = output.buffer();

    let progress_max = get_term_width() - LABEL_WIDTH - 4 - VALUES_WIDTH;

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "{:width$}", "", width = LABEL_WIDTH + 4 + progress_max).unwrap();
    writeln!(&mut stdout, "   avg10   avg60  avg300").unwrap();

    for (name, kind, stat) in rows {
        stdout.set_color(&COLOR_LABEL).unwrap();
        write!(&mut stdout, "{name:<6} {kind}").unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, " [").unwrap();

        // `avg10` is the most immediate signal, so it is the one the bar follows.
        let progress_used =
            ((stat.avg10 * progress_max as f64 / 100f64).round() as usize).min(progress_max);

        stdout.set_color(&COLOR_USED).unwrap();
        for _ in 0..progress_used {
            write!(&mut stdout, "|").unwrap();
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ").unwrap();
        }

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(&mut stdout, "] ").unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        writeln!(&mut stdout, "{:>7.2}%{:>7.2}%{:>7.2}%", stat.avg10, stat.avg60, stat.avg300)
            .unwrap();
    }

    stdout.set_color(&COLOR_DEFAULT).unwrap();

    output.print(&stdout).unwrap();

    Ok(())
}
