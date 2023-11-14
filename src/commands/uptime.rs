use mprober_lib::{format_duration, uptime};

use crate::{terminal::*, CLIArgs, CLICommands};

#[inline]
pub fn handle_uptime(args: CLIArgs) {
    debug_assert!(matches!(args.command, CLICommands::Uptime { .. }));

    if let CLICommands::Uptime {
        plain,
        light,
        monitor,
        second,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(monitor, 1000, draw_uptime(second));
    }
}

fn draw_uptime(second: bool) {
    let uptime = uptime::get_uptime().unwrap().total_uptime;

    let output = get_stdout_output();
    let mut stdout = output.buffer();

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, "This computer has been up for ").unwrap();

    if second {
        let uptime_sec = uptime.as_secs();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        write!(&mut stdout, "{uptime_sec} second").unwrap();

        if uptime_sec > 1 {
            write!(&mut stdout, "s").unwrap();
        }
    } else {
        let s = format_duration(uptime);

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        stdout.write_all(s.as_bytes()).unwrap();
    }

    stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();
    write!(&mut stdout, ".").unwrap();

    stdout.set_color(&COLOR_DEFAULT).unwrap();
    writeln!(&mut stdout).unwrap();

    output.print(&stdout).unwrap();
}
