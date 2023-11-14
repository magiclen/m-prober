use mprober_lib::rtc_time;

use crate::{terminal::*, CLIArgs, CLICommands};

#[inline]
pub fn handle_time(args: CLIArgs) {
    debug_assert!(matches!(args.command, CLICommands::Time { .. }));

    if let CLICommands::Time {
        plain,
        light,
        monitor,
    } = args.command
    {
        set_color_mode(plain, light);

        monitor_handler!(monitor, 1000, draw_time());
    }
}

fn draw_time() {
    let rtc_date_time = rtc_time::get_rtc_date_time().unwrap();

    let output = get_stdout_output();
    let mut stdout = output.buffer();

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "RTC Date").unwrap();

    write!(&mut stdout, " ").unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    write!(&mut stdout, "{}", rtc_date_time.date()).unwrap();

    writeln!(&mut stdout).unwrap();

    stdout.set_color(&COLOR_LABEL).unwrap();
    write!(&mut stdout, "RTC Time").unwrap();

    write!(&mut stdout, " ").unwrap();

    stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
    write!(&mut stdout, "{}", rtc_date_time.time()).unwrap();

    stdout.set_color(&COLOR_DEFAULT).unwrap();
    writeln!(&mut stdout).unwrap();

    output.print(&stdout).unwrap();
}
