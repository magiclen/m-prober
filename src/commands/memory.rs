use byte_unit::{Byte, Unit, UnitType};
use mprober_lib::memory;

use crate::{CLIArgs, CLICommands, terminal::*};

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

    let mem_percentage = format!("{:.2}%", percentage_of(free.mem.used, free.mem.total));
    // A machine with no swap has a total of zero, which must not turn the percentage into `NaN`.
    let swap_percentage = format!("{:.2}%", percentage_of(free.swap.used, free.swap.total));

    let percentage_len = mem_percentage.len().max(swap_percentage.len());

    let terminal_width = get_term_width();

    // Memory

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "Memory").unwrap(); // 6

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " [").unwrap(); // 2

    let progress_max =
        bar_width(terminal_width, 10 + used_len + 3 + total_len + 2 + percentage_len + 1);

    let mut remaining = progress_max;

    let progress_used = bar_cells(free.mem.used, free.mem.total, progress_max, &mut remaining);
    let progress_cache = bar_cells(free.mem.cache, free.mem.total, progress_max, &mut remaining);
    let progress_buffers =
        bar_cells(free.mem.buffers, free.mem.total, progress_max, &mut remaining);

    // Without colors the segments are told apart by their character instead.
    let (cache_cell, buffers_cell) = if is_plain_mode() { (b'$', b'#') } else { (b'|', b'|') };

    stdout.set_color(&COLOR_USED).unwrap();
    write_cells(&mut stdout, b'|', progress_used).unwrap();

    stdout.set_color(&COLOR_CACHE).unwrap();
    write_cells(&mut stdout, cache_cell, progress_cache).unwrap();

    stdout.set_color(&COLOR_BUFFERS).unwrap();
    write_cells(&mut stdout, buffers_cell, progress_buffers).unwrap();

    write_cells(&mut stdout, b' ', remaining).unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, "] ").unwrap(); // 2

    write!(&mut stdout, "{:1$}", "", used_len - mem_used.len()).unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(mem_used.as_bytes()).unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " / ").unwrap(); // 3

    write!(&mut stdout, "{:1$}", "", total_len - mem_total.len()).unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(mem_total.as_bytes()).unwrap();

    write!(&mut stdout, " (").unwrap(); // 2

    write!(&mut stdout, "{:1$}", "", percentage_len - mem_percentage.len()).unwrap();

    stdout.write_all(mem_percentage.as_bytes()).unwrap();

    write!(&mut stdout, ")").unwrap(); // 1

    writeln!(&mut stdout).unwrap();

    // Swap

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "Swap  ").unwrap(); // 6

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " [").unwrap(); // 2

    let mut remaining = progress_max;

    let progress_used = bar_cells(free.swap.used, free.swap.total, progress_max, &mut remaining);
    let progress_cache = bar_cells(free.swap.cache, free.swap.total, progress_max, &mut remaining);

    stdout.set_color(&COLOR_USED).unwrap();
    write_cells(&mut stdout, b'|', progress_used).unwrap();

    stdout.set_color(&COLOR_CACHE).unwrap();
    write_cells(&mut stdout, cache_cell, progress_cache).unwrap();

    write_cells(&mut stdout, b' ', remaining).unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, "] ").unwrap(); // 2

    write!(&mut stdout, "{:1$}", "", used_len - swap_used.len()).unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(swap_used.as_bytes()).unwrap();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, " / ").unwrap(); // 3

    write!(&mut stdout, "{:1$}", "", total_len - swap_total.len()).unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    stdout.write_all(swap_total.as_bytes()).unwrap();

    write!(&mut stdout, " (").unwrap(); // 2

    write!(&mut stdout, "{:1$}", "", percentage_len - swap_percentage.len()).unwrap();

    stdout.write_all(swap_percentage.as_bytes()).unwrap();

    write!(&mut stdout, ")").unwrap(); // 1

    stdout.set_color(&COLOR_DEFAULT).unwrap();
    writeln!(&mut stdout).unwrap();

    output.print(&stdout).unwrap();
}
