#![feature(duration_float)]

extern crate clap;
extern crate byte_unit;
extern crate validators;
extern crate termcolor;
extern crate terminal_size;
extern crate ncurses;

extern crate free;

use std::time::{Duration, SystemTime};
use std::env;
use std::path::Path;
use std::io::{self, Write};
use std::thread;
use std::sync::{Arc, Mutex};

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use terminal_size::{Width, terminal_size};
use clap::{App, Arg, SubCommand};
use validators::number::NumberGteZero;
use byte_unit::{Byte, ByteUnit};

use free::Free;

const DEFAULT_TERMINAL_WIDTH: usize = 64;
const MIN_TERMINAL_WIDTH: usize = 60;
const MIN_SLEEP_INTERVAL: u64 = 200;
const SLEEP_CHECKPOINT_COUNT: u128 = 5;

const LABEL_COLOR: Color = Color::Rgb(0, 177, 177);
const WHITE_COLOR: Color = Color::Rgb(219, 219, 219);
const RED_COLOR: Color = Color::Rgb(255, 95, 0);
const YELLOW_COLOR: Color = Color::Rgb(216, 177, 0);
const SKY_BLUE_COLOR: Color = Color::Rgb(107, 200, 200);

// TODO -----Config START-----

#[derive(Debug)]
pub enum Mode {
    Memory {
        monitor: Option<Duration>,
        plain: bool,
        unit: Option<ByteUnit>,
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
            "memory                         # Show current memory stats",
            "memory -m 1000                 # Show memory stats and refresh every 1000 milliseconds",
        ];

        let matches = App::new(APP_NAME)
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

                    ncurses::initscr();

                    thread::spawn(move || {
                        loop {
                            let key = ncurses::getch();

                            match key as u8 {
                                b'q' => {
                                    break;
                                }
                                _ => ()
                            }
                        }

                        cont_2.lock().unwrap().take();
                    });

                    let sleep_interval = Duration::from_millis(((monitor.as_millis() as u128 / SLEEP_CHECKPOINT_COUNT) as u64).max(MIN_SLEEP_INTERVAL));

                    'outer: loop {
                        let free = Free::get_free().unwrap();

                        ncurses::clear();
                        ncurses::refresh();

                        draw_free(free, !plain, unit, true).map_err(|err| err.to_string())?;

                        ncurses::refresh();

                        let s_time = SystemTime::now();

                        loop {
                            thread::sleep(sleep_interval);

                            if cont.lock().unwrap().is_none() {
                                break 'outer;
                            } else if s_time.elapsed().map_err(|err| err.to_string())? > monitor {
                                break;
                            }
                        }
                    }

                    ncurses::endwin();
                }
                None => {
                    let free = Free::get_free().unwrap();

                    draw_free(free, !plain, unit, false).map_err(|err| err.to_string())?;
                }
            }
        }
    }

    Ok(0)
}

fn draw_free(free: Free, colorful: bool, unit: Option<ByteUnit>, curses: bool) -> Result<(), io::Error> {
    let mut stdout = if colorful {
        StandardStream::stdout(ColorChoice::Always)
    } else {
        StandardStream::stdout(ColorChoice::Never)
    };

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
        write!(&mut stdout, "|")?; // 1
    }

    let progress_buffers = (free.mem.buffers as f64 * f).floor() as usize;

    stdout.set_color(ColorSpec::new().set_fg(Some(SKY_BLUE_COLOR)))?;
    for _ in 0..progress_buffers {
        write!(&mut stdout, "|")?; // 1
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

    if curses {
        stdout.flush()?;

        ncurses::addstr("\n");
    } else {
        writeln!(&mut stdout, "")?;
    }

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
        write!(&mut stdout, "|")?; // 1
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

    if curses {
        stdout.flush()?;

        ncurses::addstr("\n");
    } else {
        writeln!(&mut stdout, "")?;
    }

    Ok(())
}