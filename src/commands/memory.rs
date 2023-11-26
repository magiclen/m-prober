use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::memory;

use crate::{terminal::*, CLIArgs, CLICommands};

#[inline]
pub fn handle_memory(args: CLIArgs) {
    debug_assert!(matches!(args.command, CLICommands::Memory { .. }));

    if let CLICommands::Memory {
        plain,
        light,
        monitor,
        unit,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(monitor, draw_memory(unit));
    }
}

fn draw_memory(unit: Option<Unit>) {
    let free = memory::free().unwrap();

    let output = get_stdout_output();
    let mut stdout = output.buffer();

    let (mem_used, mem_total, swap_used, swap_total) = {
        let (mem_used, mem_total, swap_used, swap_total) = (
            Byte::from(free.mem.used),
            Byte::from(free.mem.total),
            Byte::from(free.swap.used),
            Byte::from(free.swap.total),
        );

        match unit {
            Some(unit) => (
                format!("{:.2}", mem_used.get_adjusted_unit(unit)),
                format!("{:.2}", mem_total.get_adjusted_unit(unit)),
                format!("{:.2}", swap_used.get_adjusted_unit(unit)),
                format!("{:.2}", swap_total.get_adjusted_unit(unit)),
            ),
            None => (
                format!("{:.2}", mem_used.get_appropriate_unit(UnitType::Binary)),
                format!("{:.2}", mem_total.get_appropriate_unit(UnitType::Binary)),
                format!("{:.2}", swap_used.get_appropriate_unit(UnitType::Binary)),
                format!("{:.2}", swap_total.get_appropriate_unit(UnitType::Binary)),
            ),
        }
    };

    let used_len = mem_used.len().max(swap_used.len());
    let total_len = mem_total.len().max(swap_total.len());

    let mem_percentage = format!("{:.2}%", free.mem.used as f64 * 100f64 / free.mem.total as f64);
    let swap_percentage =
        format!("{:.2}%", free.swap.used as f64 * 100f64 / free.swap.total as f64);

    let percentage_len = mem_percentage.len().max(swap_percentage.len());

    let terminal_width = get_term_width();

    // Memory

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "Memory").unwrap(); // 6

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " [").unwrap(); // 2

    let progress_max = terminal_width - 10 - used_len - 3 - total_len - 2 - percentage_len - 1;

    let f = progress_max as f64 / free.mem.total as f64;

    let progress_used = (free.mem.used as f64 * f).floor() as usize;

    stdout.set_color(&COLOR_USED).unwrap();
    for _ in 0..progress_used {
        write!(&mut stdout, "|").unwrap(); // 1
    }

    let progress_cache = (free.mem.cache as f64 * f).floor() as usize;

    stdout.set_color(&COLOR_CACHE).unwrap();
    for _ in 0..progress_cache {
        if unsafe { FORCE_PLAIN_MODE } {
            write!(&mut stdout, "$").unwrap(); // 1
        } else {
            write!(&mut stdout, "|").unwrap(); // 1
        }
    }

    let progress_buffers = (free.mem.buffers as f64 * f).floor() as usize;

    stdout.set_color(&COLOR_BUFFERS).unwrap();
    for _ in 0..progress_buffers {
        if unsafe { FORCE_PLAIN_MODE } {
            write!(&mut stdout, "#").unwrap(); // 1
        } else {
            write!(&mut stdout, "|").unwrap(); // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache - progress_buffers) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, "] ").unwrap(); // 2

    for _ in 0..(used_len - mem_used.len()) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(mem_used.as_bytes()).unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " / ").unwrap(); // 3

    for _ in 0..(total_len - mem_total.len()) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(mem_total.as_bytes()).unwrap();

    write!(&mut stdout, " (").unwrap(); // 2

    for _ in 0..(percentage_len - mem_percentage.len()) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.write_all(mem_percentage.as_bytes()).unwrap();

    write!(&mut stdout, ")").unwrap(); // 1

    writeln!(&mut stdout).unwrap();

    // Swap

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "Swap  ").unwrap(); // 6

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " [").unwrap(); // 2

    let f = progress_max as f64 / free.swap.total as f64;

    let progress_used = (free.swap.used as f64 * f).floor() as usize;

    stdout.set_color(&COLOR_USED).unwrap();
    for _ in 0..progress_used {
        write!(&mut stdout, "|").unwrap(); // 1
    }

    let progress_cache = (free.swap.cache as f64 * f).floor() as usize;

    stdout.set_color(&COLOR_CACHE).unwrap();
    for _ in 0..progress_cache {
        if unsafe { FORCE_PLAIN_MODE } {
            write!(&mut stdout, "$").unwrap(); // 1
        } else {
            write!(&mut stdout, "|").unwrap(); // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, "] ").unwrap(); // 2

    for _ in 0..(used_len - swap_used.len()) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(swap_used.as_bytes()).unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " / ").unwrap(); // 3

    for _ in 0..(total_len - swap_total.len()) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(swap_total.as_bytes()).unwrap();

    write!(&mut stdout, " (").unwrap(); // 2

    for _ in 0..(percentage_len - swap_percentage.len()) {
        write!(&mut stdout, " ").unwrap(); // 1
    }

    stdout.write_all(swap_percentage.as_bytes()).unwrap();

    write!(&mut stdout, ")").unwrap(); // 1

    stdout.set_color(&COLOR_DEFAULT).unwrap();
    writeln!(&mut stdout).unwrap();

    output.print(&stdout).unwrap();
}
