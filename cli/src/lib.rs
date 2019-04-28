#![feature(duration_float)]

extern crate clap;
extern crate byte_unit;
extern crate validators;
extern crate termcolor;
extern crate terminal_size;
extern crate getch;

extern crate free;
extern crate cpu_info;
extern crate load_average;
extern crate hostname;
extern crate time;

use std::time::{Duration, SystemTime};
use std::env;
use std::path::Path;
use std::io::{self, Write};
use std::thread;
use std::sync::{Arc, Mutex};
use std::fmt::Write as WriteFmt;

use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use terminal_size::{Width, terminal_size};
use clap::{App, Arg, SubCommand};
use validators::number::NumberGteZero;
use byte_unit::{Byte, ByteUnit};
use getch::Getch;

use free::Free;
use cpu_info::{CPU, CPUStat};
use load_average::LoadAverage;
use time::RTCDateTime;

const DEFAULT_TERMINAL_WIDTH: usize = 64;
const MIN_TERMINAL_WIDTH: usize = 60;
const MIN_SLEEP_INTERVAL: u64 = 200;
const MAX_SLEEP_INTERVAL: u64 = 5000;
const SLEEP_CHECKPOINT_COUNT: u128 = 5;

const LABEL_COLOR: Color = Color::Rgb(0, 177, 177);
const WHITE_COLOR: Color = Color::Rgb(219, 219, 219);
const RED_COLOR: Color = Color::Rgb(255, 95, 0);
const YELLOW_COLOR: Color = Color::Rgb(216, 177, 0);
const SKY_BLUE_COLOR: Color = Color::Rgb(107, 200, 200);

const CLEAR_SCREEN_DATA: [u8; 11] = [0x1b, 0x5b, 0x33, 0x4a, 0x1b, 0x5b, 0x48, 0x1b, 0x5b, 0x32, 0x4a];

// TODO -----Config START-----

#[derive(Debug)]
pub enum Mode {
    Memory {
        monitor: Option<Duration>,
        plain: bool,
        unit: Option<ByteUnit>,
    },
    CPU {
        monitor: Option<Duration>,
        plain: bool,
        separate: bool,
        information: bool,
    },
    HostName,
    Uptime {
        monitor: bool,
        plain: bool,
        second: bool,
    },
    Time {
        monitor: bool,
        plain: bool,
    },
}

#[derive(Debug)]
pub struct Config {
    pub mode: Mode
}

const APP_NAME: &str = "MagicLen Prober";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

impl Config {
    pub fn from_cli() -> Result<Config, String> {
        let arg0 = env::args().next().unwrap();
        let arg0 = Path::new(&arg0).file_stem().unwrap().to_str().unwrap();

        let examples = vec![
            "memory                      # Show current memory stats",
            "memory -m 1000              # Show memory stats and refresh every 1000 milliseconds",
            "memory -p                   # Show memory stats without colors",
            "memory -u kb                # Show memory stats in KB",
            "cpu                         # Show load average and CPU stats on average",
            "cpu -m 1000                 # Show load average and CPU stats on average and refresh every 1000 milliseconds",
            "cpu -p                      # Show load average and CPU stats on average without colors",
            "cpu -s                      # Show load average and stats of CPU cores separately",
            "cpu -i                      # Only show CPU information",
            "hostname                    # Show the hostname",
            "uptime                      # Show the uptime",
            "uptime -m                   # Show the uptime and refresh every second",
            "uptime -p                   # Show the uptime without colors",
            "uptime -s                   # Show the uptime in seconds",
            "time                        # Show the RTC (UTC) date and time",
            "time -m                     # Show the RTC (UTC) date and time and refresh every second",
            "time -p                     # Show the RTC (UTC) date and time without colors",
        ];

        let terminal_width = if let Some((Width(width), _)) = terminal_size() {
            (width as usize).max(MIN_TERMINAL_WIDTH)
        } else {
            DEFAULT_TERMINAL_WIDTH
        };

        let matches = App::new(APP_NAME)
            .set_term_width(terminal_width)
            .version(CARGO_PKG_VERSION)
            .author(CARGO_PKG_AUTHORS)
            .about(format!("MagicLen Prober is a free and simple probe utility for Linux.\n\nEXAMPLES:\n{}", examples.iter()
                .map(|e| format!("  {} {}\n", arg0, e))
                .collect::<Vec<String>>()
                .concat()
            ).as_str()
            )
            .subcommand(SubCommand::with_name("memory").aliases(&["m", "mem", "free", "memories", "swap", "ram", "dram", "ddr", "cache", "buffer", "buf"])
                .about("Shows memory stats")
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows memory stats and refreshes every N milliseconds")
                    .takes_value(true)
                    .value_name("MILLI_SECONDS")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("UNIT")
                    .long("unit")
                    .short("u")
                    .help("Forces to use a fixed unit")
                    .takes_value(true)
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("cpu").aliases(&["c", "cpus", "core", "cores", "load", "processor", "processors"])
                .about("Shows CPU stats")
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows CPU stats and refreshes every N milliseconds")
                    .takes_value(true)
                    .value_name("MILLI_SECONDS")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("SEPARATE")
                    .long("separate")
                    .short("s")
                    .help("Separates each CPU")
                )
                .arg(Arg::with_name("INFORMATION")
                    .long("information")
                    .short("i")
                    .help("Shows only information about CPUs")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("hostname").aliases(&["n", "h", "host", "name", "servername", "server"])
                .about("Shows the hostname")
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("uptime").aliases(&["u", "up", "utime", "ut"])
                .about("Shows the uptime")
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows the uptime and refreshes every second")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .arg(Arg::with_name("SECOND")
                    .long("second")
                    .short("s")
                    .help("Shows the uptime in seconds")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("time").aliases(&["t", "systime", "stime", "st", "utc", "utctime", "rtc", "rtctime", "date"])
                .about("Shows the RTC (UTC) date and time")
                .arg(Arg::with_name("MONITOR")
                    .long("monitor")
                    .short("m")
                    .help("Shows the RTC (UTC) date and time, and refreshes every second")
                )
                .arg(Arg::with_name("PLAIN")
                    .long("plain")
                    .short("p")
                    .help("No colors")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let mode = if let Some(sub_matches) = matches.subcommand_matches("memory") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGteZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let plain = sub_matches.is_present("PLAIN");

            let unit = match sub_matches.value_of("UNIT") {
                Some(unit) => {
                    let unit = ByteUnit::from_str(unit).map_err(|_| format!("`{}` is not a correct value for UNIT", unit))?;

                    Some(unit)
                }
                None => None
            };

            Mode::Memory {
                monitor,
                plain,
                unit,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("cpu") {
            let monitor = match sub_matches.value_of("MONITOR") {
                Some(monitor) => {
                    let monitor = NumberGteZero::from_str(monitor).map_err(|_| format!("`{}` is not a correct value for MILLI_SECONDS", monitor))?.get_number();

                    Some(Duration::from_secs_f64(monitor / 1000f64))
                }
                None => None
            };

            let plain = sub_matches.is_present("PLAIN");

            let separate = sub_matches.is_present("SEPARATE");

            let information = sub_matches.is_present("INFORMATION");

            Mode::CPU {
                monitor,
                plain,
                separate,
                information,
            }
        } else if let Some(_) = matches.subcommand_matches("hostname") {
            Mode::HostName
        } else if let Some(sub_matches) = matches.subcommand_matches("uptime") {
            let monitor = sub_matches.is_present("MONITOR");

            let plain = sub_matches.is_present("PLAIN");

            let second = sub_matches.is_present("SECOND");

            Mode::Uptime {
                monitor,
                plain,
                second,
            }
        } else if let Some(sub_matches) = matches.subcommand_matches("time") {
            let monitor = sub_matches.is_present("MONITOR");

            let plain = sub_matches.is_present("PLAIN");

            Mode::Time {
                monitor,
                plain,
            }
        } else {
            return Err(String::from("Please input a subcommand. Use `help` to see how to use this program."));
        };

        let config = Config {
            mode
        };

        Ok(config)
    }
}

// TODO -----Config END----

pub fn run(config: Config) -> Result<i32, String> {
    let mode = config.mode;

    match mode {
        Mode::Memory { monitor, plain, unit } => {
            match monitor {
                Some(monitor) => {
                    let cont = Arc::new(Mutex::new(Some(0)));
                    let cont_2 = cont.clone();

                    thread::spawn(move || {
                        loop {
                            let key = Getch::new().getch().unwrap();

                            match key {
                                b'q' => {
                                    break;
                                }
                                _ => ()
                            }
                        }

                        cont_2.lock().unwrap().take();
                    });

                    let sleep_interval = Duration::from_millis(((monitor.as_millis() as u128 / SLEEP_CHECKPOINT_COUNT) as u64).max(MIN_SLEEP_INTERVAL).min(MAX_SLEEP_INTERVAL));

                    'memory_outer: loop {
                        let s_time = SystemTime::now();

                        draw_free(!plain, unit, true).map_err(|err| err.to_string())?;

                        loop {
                            thread::sleep(sleep_interval);

                            if cont.lock().unwrap().is_none() {
                                break 'memory_outer;
                            } else if s_time.elapsed().map_err(|err| err.to_string())? > monitor {
                                break;
                            }
                        }
                    }
                }
                None => {
                    draw_free(!plain, unit, false).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::CPU { monitor, plain, separate, information } => {
            match monitor {
                Some(monitor) => {
                    let cont = Arc::new(Mutex::new(Some(0)));
                    let cont_2 = cont.clone();

                    thread::spawn(move || {
                        loop {
                            let key = Getch::new().getch().unwrap();

                            match key {
                                b'q' => {
                                    break;
                                }
                                _ => ()
                            }
                        }

                        cont_2.lock().unwrap().take();
                    });

                    let sleep_interval = Duration::from_millis(((monitor.as_millis() as u128 / SLEEP_CHECKPOINT_COUNT) as u64).max(MIN_SLEEP_INTERVAL).min(MAX_SLEEP_INTERVAL));

                    'cpu_outer: loop {
                        let s_time = SystemTime::now();

                        draw_cpu_info(!plain, separate, information, true).map_err(|err| err.to_string())?;

                        loop {
                            thread::sleep(sleep_interval);

                            if cont.lock().unwrap().is_none() {
                                break 'cpu_outer;
                            } else if s_time.elapsed().map_err(|err| err.to_string())? > monitor {
                                break;
                            }
                        }
                    }
                }
                None => {
                    draw_cpu_info(!plain, separate, information, false).map_err(|err| err.to_string())?;
                }
            }
        }
        Mode::HostName => {
            let hostname = hostname::get_hostname().map_err(|err| err.to_string())?;

            println!("{}", hostname);
        }
        Mode::Uptime { monitor, plain, second } => {
            if monitor {
                let cont = Arc::new(Mutex::new(Some(0)));
                let cont_2 = cont.clone();

                thread::spawn(move || {
                    loop {
                        let key = Getch::new().getch().unwrap();

                        match key {
                            b'q' => {
                                break;
                            }
                            _ => ()
                        }
                    }

                    cont_2.lock().unwrap().take();
                });

                let monitor = Duration::from_secs(1);

                let sleep_interval = Duration::from_millis(((monitor.as_millis() as u128 / SLEEP_CHECKPOINT_COUNT) as u64).max(MIN_SLEEP_INTERVAL).min(MAX_SLEEP_INTERVAL));

                'uptime_outer: loop {
                    let s_time = SystemTime::now();

                    draw_uptime(!plain, second, true).map_err(|err| err.to_string())?;

                    loop {
                        thread::sleep(sleep_interval);

                        if cont.lock().unwrap().is_none() {
                            break 'uptime_outer;
                        } else if s_time.elapsed().map_err(|err| err.to_string())? > monitor {
                            break;
                        }
                    }
                }
            } else {
                draw_uptime(!plain, second, false).map_err(|err| err.to_string())?;
            }
        }
        Mode::Time { monitor, plain } => {
            if monitor {
                let cont = Arc::new(Mutex::new(Some(0)));
                let cont_2 = cont.clone();

                thread::spawn(move || {
                    loop {
                        let key = Getch::new().getch().unwrap();

                        match key {
                            b'q' => {
                                break;
                            }
                            _ => ()
                        }
                    }

                    cont_2.lock().unwrap().take();
                });

                let monitor = Duration::from_secs(1);

                let sleep_interval = Duration::from_millis(((monitor.as_millis() as u128 / SLEEP_CHECKPOINT_COUNT) as u64).max(MIN_SLEEP_INTERVAL).min(MAX_SLEEP_INTERVAL));

                'time_outer: loop {
                    let s_time = SystemTime::now();

                    draw_time(!plain, true).map_err(|err| err.to_string())?;

                    loop {
                        thread::sleep(sleep_interval);

                        if cont.lock().unwrap().is_none() {
                            break 'time_outer;
                        } else if s_time.elapsed().map_err(|err| err.to_string())? > monitor {
                            break;
                        }
                    }
                }
            } else {
                draw_time(!plain, false).map_err(|err| err.to_string())?;
            }
        }
//        _ => unreachable!()
    }

    Ok(0)
}

fn draw_free(colorful: bool, unit: Option<ByteUnit>, monitor: bool) -> Result<(), io::Error> {
    let free = Free::get_free().unwrap();

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    let (mem_used, mem_total, swap_used, swap_total) = {
        let (mem_used, mem_total, swap_used, swap_total) = (
            Byte::from_bytes(free.mem.used as u128),
            Byte::from_bytes(free.mem.total as u128),
            Byte::from_bytes(free.swap.used as u128),
            Byte::from_bytes(free.swap.total as u128),
        );

        match unit {
            Some(unit) => {
                (
                    mem_used.get_adjusted_unit(unit).to_string(),
                    mem_total.get_adjusted_unit(unit).to_string(),
                    swap_used.get_adjusted_unit(unit).to_string(),
                    swap_total.get_adjusted_unit(unit).to_string(),
                )
            }
            None => {
                (
                    mem_used.get_appropriate_unit(true).to_string(),
                    mem_total.get_appropriate_unit(true).to_string(),
                    swap_used.get_appropriate_unit(true).to_string(),
                    swap_total.get_appropriate_unit(true).to_string(),
                )
            }
        }
    };

    let used_len = mem_used.len().max(swap_used.len());
    let total_len = mem_total.len().max(swap_total.len());

    let mem_percentage = format!("{:.2}%", free.mem.used as f64 * 100f64 / free.mem.total as f64);
    let swap_percentage = format!("{:.2}%", free.swap.used as f64 * 100f64 / free.swap.total as f64);

    let percentage_len = mem_percentage.len().max(swap_percentage.len());

    let terminal_width = if let Some((Width(width), _)) = terminal_size() {
        (width as usize).max(MIN_TERMINAL_WIDTH)
    } else {
        DEFAULT_TERMINAL_WIDTH
    };

    // Memory

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "Memory")?; // 6

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " [")?; // 2

    let progress_max = terminal_width - 10 - used_len - 3 - total_len - 2 - percentage_len - 1;

    let f = progress_max as f64 / free.mem.total as f64;

    let progress_used = (free.mem.used as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
    for _ in 0..progress_used {
        write!(&mut stdout, "|")?; // 1
    }

    let progress_cache = (free.mem.cache as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(YELLOW_COLOR)))?;
    for _ in 0..progress_cache {
        if colorful {
            write!(&mut stdout, "|")?; // 1
        } else {
            write!(&mut stdout, "$")?; // 1
        }
    }

    let progress_buffers = (free.mem.buffers as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(SKY_BLUE_COLOR)))?;
    for _ in 0..progress_buffers {
        if colorful {
            write!(&mut stdout, "|")?; // 1
        } else {
            write!(&mut stdout, "#")?; // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache - progress_buffers) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, "] ")?; // 2

    for _ in 0..(used_len - mem_used.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(mem_used.as_bytes())?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " / ")?; // 3

    for _ in 0..(total_len - mem_total.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(mem_total.as_bytes())?;

    write!(&mut stdout, " (")?; // 2

    for _ in 0..(percentage_len - mem_percentage.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.write_all(mem_percentage.as_bytes())?;

    write!(&mut stdout, ")")?; // 1

    writeln!(&mut stdout, "")?;

    // Swap

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "Swap  ")?; // 6

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " [")?; // 2

    let f = progress_max as f64 / free.swap.total as f64;

    let progress_used = (free.swap.used as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
    for _ in 0..progress_used {
        write!(&mut stdout, "|")?; // 1
    }

    let progress_cache = (free.swap.cache as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(YELLOW_COLOR)))?;
    for _ in 0..progress_cache {
        if colorful {
            write!(&mut stdout, "|")?; // 1
        } else {
            write!(&mut stdout, "$")?; // 1
        }
    }

    for _ in 0..(progress_max - progress_used - progress_cache) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, "] ")?; // 2

    for _ in 0..(used_len - swap_used.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(swap_used.as_bytes())?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, " / ")?; // 3

    for _ in 0..(total_len - swap_total.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(swap_total.as_bytes())?;

    write!(&mut stdout, " (")?; // 2

    for _ in 0..(percentage_len - swap_percentage.len()) {
        write!(&mut stdout, " ")?; // 1
    }

    stdout.write_all(swap_percentage.as_bytes())?;

    write!(&mut stdout, ")")?; // 1

    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

fn draw_cpu_info(colorful: bool, separate: bool, only_information: bool, monitor: bool) -> Result<(), io::Error> {
    let cpus: Vec<CPU> = CPU::get_cpus().unwrap();

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    let terminal_width = if let Some((Width(width), _)) = terminal_size() {
        (width as usize).max(MIN_TERMINAL_WIDTH)
    } else {
        DEFAULT_TERMINAL_WIDTH
    };

    // load average
    if !only_information {
        let load_average: LoadAverage = LoadAverage::get_load_average().unwrap();

        let logical_cores_number: usize = cpus.iter().map(|cpu| cpu.siblings).sum();
        let logical_cores_number_f64 = logical_cores_number as f64;

        let one = format!("{:.2}", load_average.one);
        let five = format!("{:.2}", load_average.five);
        let fifteen = format!("{:.2}", load_average.fifteen);

        let load_average_len = one.len().max(five.len()).max(fifteen.len());

        let one_percentage = format!("{:.2}%", load_average.one * 100f64 / logical_cores_number_f64);
        let five_percentage = format!("{:.2}%", load_average.five * 100f64 / logical_cores_number_f64);
        let fifteen_percentage = format!("{:.2}%", load_average.fifteen * 100f64 / logical_cores_number_f64);

        let percentage_len = one_percentage.len().max(five_percentage.len()).max(fifteen_percentage.len());

        let progress_max = terminal_width - 11 - load_average_len - 2 - percentage_len - 1;

        // number of logical CPU cores

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        if logical_cores_number > 1 {
            write!(&mut stdout, "There are ")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            write!(&mut stdout, "{}", logical_cores_number)?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, " logical CPU cores.")?;
        } else {
            write!(&mut stdout, "There is only one logical CPU core.")?;
        }
        writeln!(&mut stdout, "")?;

        // one

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "one    ")?; // 7

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.one * f).floor() as usize).min(progress_max);

        stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - one.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(one.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - one_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(one_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout, "")?;

        // five

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "five   ")?; // 7

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.five * f).floor() as usize).min(progress_max);

        stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - five.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(five.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - five_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(five_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout, "")?;

        // fifteen

        stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
        write!(&mut stdout, "fifteen")?; // 7

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, " [")?; // 2

        let f = progress_max as f64 / logical_cores_number_f64;

        let progress_used = ((load_average.fifteen * f).floor() as usize).min(progress_max);

        stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
        for _ in 0..progress_used {
            write!(&mut stdout, "|")?; // 1
        }

        for _ in 0..(progress_max - progress_used) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
        write!(&mut stdout, "] ")?; // 2

        for _ in 0..(load_average_len - fifteen.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(fifteen.as_bytes())?;

        write!(&mut stdout, " (")?; // 2

        for _ in 0..(percentage_len - fifteen_percentage.len()) {
            write!(&mut stdout, " ")?; // 1
        }

        stdout.write_all(fifteen_percentage.as_bytes())?;

        write!(&mut stdout, ")")?; // 1

        writeln!(&mut stdout, "")?;
        writeln!(&mut stdout, "")?;
    }

    // information

    if separate {
        let all_percentage: Vec<f64> = if only_information {
            Vec::new()
        } else {
            CPUStat::get_all_percentage(Duration::from_millis(MIN_SLEEP_INTERVAL)).unwrap()
        };

        let mut i = 0;

        let cpus_len_dec = cpus.len() - 1;

        for (cpu_index, cpu) in cpus.into_iter().enumerate() {
            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            writeln!(&mut stdout, "")?;

            let mut hz_string: Vec<String> = Vec::with_capacity(cpu.siblings);

            for &cpu_mhz in cpu.cpus_mhz.iter() {
                let cpu_hz = Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

                hz_string.push(format!("{:.2}{}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1]));
            }

            let hz_string_len = hz_string.iter().map(|s| s.len()).max().unwrap();

            let d = {
                let mut n = cpu.siblings;

                let mut d = 1;

                while n > 10 {
                    n /= 10;

                    d += 1;
                }

                d
            };

            if only_information {
                for (i, hz_string) in hz_string.into_iter().enumerate() {
                    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
                    write!(&mut stdout, "{1:<0$}", d + 4, format!("CPU{}", i))?;

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
                    write!(&mut stdout, "{1:>0$}", hz_string_len, hz_string)?;

                    writeln!(&mut stdout, "")?;
                }
            } else {
                let mut percentage_string: Vec<String> = Vec::with_capacity(cpu.siblings);

                for &p in all_percentage[i..].iter().take(cpu.siblings) {
                    percentage_string.push(format!("{:.2}%", p * 100f64));
                }

                let percentage_len = percentage_string.iter().map(|s| s.len()).max().unwrap();

                let progress_max = terminal_width - d - 7 - percentage_len - 2 - hz_string_len - 1;

                for (i, &p) in all_percentage[i..].iter().take(cpu.siblings).enumerate() {
                    let percentage_string = &percentage_string[i];
                    let hz_string = &hz_string[i];

                    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
                    write!(&mut stdout, "CPU")?; // 3

                    write!(&mut stdout, "{}", i)?;

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
                    write!(&mut stdout, " [")?; // 2

                    let f = progress_max as f64;

                    let progress_used = (p * f).floor() as usize;

                    stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
                    for _ in 0..progress_used {
                        write!(&mut stdout, "|")?; // 1
                    }

                    for _ in 0..(progress_max - progress_used) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
                    write!(&mut stdout, "] ")?; // 2

                    for _ in 0..(percentage_len - percentage_string.len()) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
                    stdout.write_all(percentage_string.as_bytes())?;

                    write!(&mut stdout, " (")?; // 2

                    for _ in 0..(hz_string_len - hz_string.len()) {
                        write!(&mut stdout, " ")?; // 1
                    }

                    stdout.write_all(hz_string.as_bytes())?;

                    write!(&mut stdout, ")")?; // 1

                    writeln!(&mut stdout, "")?;
                }

                i += cpu.siblings;
            }

            if cpu_index != cpus_len_dec {
                writeln!(&mut stdout, "")?;
            }
        }
    } else {
        let (average_percentage, average_percentage_string) = if only_information {
            (0f64, "".to_string())
        } else {
            let average_percentage = CPUStat::get_average_percentage(Duration::from_millis(MIN_SLEEP_INTERVAL)).unwrap();

            let average_percentage_string = format!("{:.2}%", average_percentage * 100f64);

            (average_percentage, average_percentage_string)
        };

        for cpu in cpus {
            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            stdout.write_all(cpu.model_name.as_bytes())?;

            write!(&mut stdout, " ")?;

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;

            write!(&mut stdout, "{}C/{}T", cpu.cpu_cores, cpu.siblings)?;

            write!(&mut stdout, " ")?;

            let cpu_mhz: f64 = cpu.cpus_mhz.iter().sum::<f64>() / cpu.cpus_mhz.len() as f64;

            let cpu_hz = Byte::from_unit(cpu_mhz, ByteUnit::MB).unwrap().get_appropriate_unit(false);

            write!(&mut stdout, "{:.2}{}Hz", cpu_hz.get_value(), &cpu_hz.get_unit().as_str()[..1])?;

            writeln!(&mut stdout, "")?;
        }

        if !only_information {
            let progress_max = terminal_width - 7 - average_percentage_string.len();

            stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
            write!(&mut stdout, "CPU")?; // 3

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, " [")?; // 2

            let f = progress_max as f64;

            let progress_used = (average_percentage * f).floor() as usize;

            stdout.set_color(ColorSpec::new().set_fg(Some(RED_COLOR)))?;
            for _ in 0..progress_used {
                write!(&mut stdout, "|")?; // 1
            }

            for _ in 0..(progress_max - progress_used) {
                write!(&mut stdout, " ")?; // 1
            }

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
            write!(&mut stdout, "] ")?; // 2

            stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
            stdout.write_all(average_percentage_string.as_bytes())?;

            writeln!(&mut stdout, "")?;
        }
    }

    output.print(&stdout)?;

    Ok(())
}

fn draw_uptime(colorful: bool, second: bool, monitor: bool) -> Result<(), io::Error> {
    let uptime = time::get_uptime().unwrap();

    let uptime_sec = uptime.as_secs();

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, "This computer has been up for ")?;

    if second {
        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        write!(&mut stdout, "{} second", uptime_sec)?;

        if uptime_sec > 1 {
            write!(&mut stdout, "s")?;
        }
    } else {
        let days = uptime_sec / 86400;

        let uptime_sec = uptime_sec % 86400;

        let hours = uptime_sec / 3600;

        let uptime_sec = uptime_sec % 3600;

        let minutes = uptime_sec / 60;

        let seconds = uptime_sec % 60;

        let mut s = String::new();

        if days > 0 {
            s.write_fmt(format_args!("{} day", days)).unwrap();

            if days > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        if hours > 0 || (days > 0) && (minutes > 0 || seconds > 0) {
            s.write_fmt(format_args!("{} hour", hours)).unwrap();

            if hours > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        if minutes > 0 || (hours > 0 && seconds > 0) {
            s.write_fmt(format_args!("{} minute", minutes)).unwrap();

            if minutes > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        if seconds > 0 {
            s.write_fmt(format_args!("{} second", seconds)).unwrap();

            if seconds > 1 {
                s.push('s');
            }

            s.push_str(", ");
        }

        debug_assert!(s.len() >= 2);

        if let Some(index) = s.as_str()[..(s.len() - 2)].rfind(", ") {
            s.insert_str(index + 2, "and ");
        }

        let b = s.as_bytes();

        stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
        stdout.write_all(&b[..(b.len() - 2)])?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)))?;
    write!(&mut stdout, ".")?;

    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}

fn draw_time(colorful: bool, monitor: bool) -> Result<(), io::Error> {
    let rtc_date_time: RTCDateTime = RTCDateTime::get_rtc_date_time().unwrap();

    let output = if colorful {
        BufferWriter::stdout(ColorChoice::Always)
    } else {
        BufferWriter::stdout(ColorChoice::Never)
    };

    let mut stdout = output.buffer();

    if monitor {
        stdout.write_all(&CLEAR_SCREEN_DATA)?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "RTC Date")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(rtc_date_time.rtc_date.as_bytes())?;

    writeln!(&mut stdout, "")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(LABEL_COLOR)))?;
    write!(&mut stdout, "RTC Time")?;

    write!(&mut stdout, " ")?;

    stdout.set_color(ColorSpec::new().set_fg(Some(WHITE_COLOR)).set_bold(true))?;
    stdout.write_all(rtc_date_time.rtc_time.as_bytes())?;

    writeln!(&mut stdout, "")?;

    output.print(&stdout)?;

    Ok(())
}