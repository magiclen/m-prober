use anyhow::anyhow;
use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::{cgroup, format_duration};

use crate::{CLIArgs, CLICommands, terminal::*};

/// The width of `Memory`, which is the longest label of a bar row.
const LABEL_WIDTH: usize = 6;

#[inline]
pub fn handle_cgroup(args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(args.command, CLICommands::Cgroup { .. }));

    if let CLICommands::Cgroup {
        plain,
        light,
        monitor,
        unit,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(monitor, draw_cgroup(unit)?);
    }

    Ok(())
}

fn draw_cgroup(unit: Option<Unit>) -> anyhow::Result<()> {
    let path = cgroup::get_cgroup_path().map_err(|_| {
        anyhow!("This process does not run under cgroup v2, so there are no limits to show.")
    })?;

    let output = get_stdout_output();
    let mut stdout = output.buffer();

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "cgroup ").unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    writeln!(&mut stdout, "{}", path.display()).unwrap();
    writeln!(&mut stdout).unwrap();

    let terminal_width = get_term_width();

    if let Some(cpu) = optional(cgroup::get_cgroup_cpu(&path))? {
        stdout.set_color(&COLOR_LABEL).unwrap();
        writeln!(&mut stdout, "CPU").unwrap();

        let limit = match cpu.effective_cpu_count() {
            Some(cpus) => format!("{cpus:.2} CPUs"),
            None => String::from("not limited"),
        };

        draw_field(&mut stdout, "limit", &limit);
        draw_field(&mut stdout, "usage", &format_duration(cpu.usage));
        draw_field(&mut stdout, "user", &format_duration(cpu.user));
        draw_field(&mut stdout, "system", &format_duration(cpu.system));

        // The ratio of throttled to elapsed periods is what tells whether the quota is actually in the way.
        draw_field(
            &mut stdout,
            "throttled",
            &format!(
                "{} of {} periods, for {}",
                cpu.nr_throttled,
                cpu.nr_periods,
                format_duration(cpu.throttled)
            ),
        );

        writeln!(&mut stdout).unwrap();
    }

    if let Some(memory) = optional(cgroup::get_cgroup_memory(&path))? {
        draw_usage(
            &mut stdout,
            "Memory",
            memory.current,
            memory.max,
            unit,
            terminal_width,
            UnitType::Binary,
        );

        if let Some(swap_current) = memory.swap_current {
            draw_usage(
                &mut stdout,
                "Swap",
                swap_current,
                memory.swap_max,
                unit,
                terminal_width,
                UnitType::Binary,
            );
        }
    }

    if let Some(pids) = optional(cgroup::get_cgroup_pids(&path))? {
        draw_count(&mut stdout, "PIDs", pids.current, pids.max, terminal_width);
    }

    stdout.set_color(&COLOR_DEFAULT).unwrap();

    output.print(&stdout).unwrap();

    Ok(())
}

fn draw_field(stdout: &mut impl WriteColor, name: &str, value: &str) {
    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(stdout, "  {name:<10}").unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    writeln!(stdout, "{value}").unwrap();
}

/// A controller which is not enabled for the cgroup has no files, which is not an error here.
#[inline]
fn optional<T>(result: Result<T, mprober_lib::Error>) -> Result<Option<T>, mprober_lib::Error> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.is_not_supported() => Ok(None),
        Err(error) => Err(error),
    }
}

fn draw_usage(
    stdout: &mut impl WriteColor,
    label: &str,
    current: u64,
    max: Option<u64>,
    unit: Option<Unit>,
    terminal_width: usize,
    unit_type: UnitType,
) {
    let format_byte = |value: u64| match unit {
        Some(unit) => format!("{:.2}", Byte::from(value).get_adjusted_unit(unit)),
        None => format!("{:.2}", Byte::from(value).get_appropriate_unit(unit_type)),
    };

    draw_bar(
        stdout,
        label,
        current,
        max,
        format_byte(current),
        max.map(format_byte),
        terminal_width,
    );
}

fn draw_count(
    stdout: &mut impl WriteColor,
    label: &str,
    current: u64,
    max: Option<u64>,
    terminal_width: usize,
) {
    draw_bar(
        stdout,
        label,
        current,
        max,
        current.to_string(),
        max.map(|max| max.to_string()),
        terminal_width,
    );
}

fn draw_bar(
    stdout: &mut impl WriteColor,
    label: &str,
    current: u64,
    max: Option<u64>,
    current_text: String,
    max_text: Option<String>,
    terminal_width: usize,
) {
    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(stdout, "{label:<LABEL_WIDTH$}").unwrap();

    let Some((max, max_text)) = max.zip(max_text) else {
        // Without a limit there is nothing to fill a bar up to, so only the current value is shown.
        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        write!(stdout, " ").unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        write!(stdout, "{current_text}").unwrap();

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
        writeln!(stdout, ", not limited").unwrap();

        return;
    };

    let percentage = format!("{:.2}%", percentage_of(current, max));

    let values_width = current_text.len() + 3 + max_text.len() + 2 + percentage.len() + 1;
    let progress_max = bar_width(terminal_width, LABEL_WIDTH + 4 + values_width);

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(stdout, " [").unwrap();

    let mut remaining = progress_max;

    let progress_used = bar_cells(current, max, progress_max, &mut remaining);

    stdout.set_color(&COLOR_USED).unwrap();
    write_cells(stdout, b'|', progress_used).unwrap();

    write_cells(stdout, b' ', remaining).unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(stdout, "] ").unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    write!(stdout, "{current_text}").unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(stdout, " / ").unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    write!(stdout, "{max_text}").unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    writeln!(stdout, " ({percentage})").unwrap();
}
