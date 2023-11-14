use std::env;
pub use std::{io::Write, time::Duration};

use once_cell::sync::Lazy;
pub use termcolor::WriteColor;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec};
use terminal_size::terminal_size;

// dark mode
const CYAN_COLOR: Color = Color::Rgb(0, 177, 177);
const WHITE_COLOR: Color = Color::Rgb(219, 219, 219);
const RED_COLOR: Color = Color::Rgb(255, 95, 0);
const YELLOW_COLOR: Color = Color::Rgb(216, 177, 0);
const SKY_CYAN_COLOR: Color = Color::Rgb(107, 200, 200);

// light mode
const DARK_CYAN_COLOR: Color = Color::Rgb(0, 95, 95);
const BLACK_COLOR: Color = Color::Rgb(28, 28, 28);
const WINE_COLOR: Color = Color::Rgb(215, 0, 0);
const ORANGE_COLOR: Color = Color::Rgb(215, 135, 0);
const DARK_BLUE_COLOR: Color = Color::Rgb(0, 0, 95);

pub const CLEAR_SCREEN_DATA: [u8; 11] =
    [0x1B, 0x5B, 0x33, 0x4A, 0x1B, 0x5B, 0x48, 0x1B, 0x5B, 0x32, 0x4A];

const ENV_LIGHT_MODE: &str = "MPROBER_LIGHT";
const ENV_FORCE_PLAIN: &str = "MPROBER_FORCE_PLAIN";

pub const DEFAULT_TERMINAL_WIDTH: usize = 80;
pub const MIN_TERMINAL_WIDTH: usize = 60;

pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(333); // should be smaller than 1000 milliseconds

pub static mut FORCE_PLAIN_MODE: bool = false;
static mut LIGHT_MODE: bool = false;

pub static COLOR_DEFAULT: Lazy<ColorSpec> = Lazy::new(ColorSpec::new);

pub static COLOR_LABEL: Lazy<ColorSpec> = Lazy::new(|| {
    let mut color_spec = ColorSpec::new();

    if !unsafe { FORCE_PLAIN_MODE } {
        if unsafe { LIGHT_MODE } {
            color_spec.set_fg(Some(DARK_CYAN_COLOR));
        } else {
            color_spec.set_fg(Some(CYAN_COLOR));
        }
    }

    color_spec
});

pub static COLOR_NORMAL_TEXT: Lazy<ColorSpec> = Lazy::new(|| {
    let mut color_spec = ColorSpec::new();

    if !unsafe { FORCE_PLAIN_MODE } {
        if unsafe { LIGHT_MODE } {
            color_spec.set_fg(Some(BLACK_COLOR));
        } else {
            color_spec.set_fg(Some(WHITE_COLOR));
        }
    }

    color_spec
});

pub static COLOR_BOLD_TEXT: Lazy<ColorSpec> = Lazy::new(|| {
    let mut color_spec = ColorSpec::new();

    if !unsafe { FORCE_PLAIN_MODE } {
        if unsafe { LIGHT_MODE } {
            color_spec.set_fg(Some(BLACK_COLOR)).set_bold(true);
        } else {
            color_spec.set_fg(Some(WHITE_COLOR)).set_bold(true);
        }
    }

    color_spec
});

pub static COLOR_USED: Lazy<ColorSpec> = Lazy::new(|| {
    let mut color_spec = ColorSpec::new();

    if !unsafe { FORCE_PLAIN_MODE } {
        if unsafe { LIGHT_MODE } {
            color_spec.set_fg(Some(WINE_COLOR));
        } else {
            color_spec.set_fg(Some(RED_COLOR));
        }
    }

    color_spec
});

pub static COLOR_CACHE: Lazy<ColorSpec> = Lazy::new(|| {
    let mut color_spec = ColorSpec::new();

    if !unsafe { FORCE_PLAIN_MODE } {
        if unsafe { LIGHT_MODE } {
            color_spec.set_fg(Some(ORANGE_COLOR));
        } else {
            color_spec.set_fg(Some(YELLOW_COLOR));
        }
    }

    color_spec
});

pub static COLOR_BUFFERS: Lazy<ColorSpec> = Lazy::new(|| {
    let mut color_spec = ColorSpec::new();

    if !unsafe { FORCE_PLAIN_MODE } {
        if unsafe { LIGHT_MODE } {
            color_spec.set_fg(Some(DARK_BLUE_COLOR));
        } else {
            color_spec.set_fg(Some(SKY_CYAN_COLOR));
        }
    }

    color_spec
});

pub fn set_color_mode(plain: bool, light: bool) {
    unsafe {
        if plain {
            FORCE_PLAIN_MODE = true;
        } else {
            match env::var_os(ENV_FORCE_PLAIN).map(|v| v.ne("0")) {
                Some(true) => {
                    FORCE_PLAIN_MODE = true;
                },
                _ => {
                    if light {
                        LIGHT_MODE = true;
                    } else {
                        LIGHT_MODE =
                            env::var_os(ENV_LIGHT_MODE).map(|v| v.ne("0")).unwrap_or(false);
                    }
                },
            }
        }
    }
}

pub fn get_stdout_output() -> BufferWriter {
    if unsafe { FORCE_PLAIN_MODE } {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    }
}

pub fn get_term_width() -> usize {
    terminal_size()
        .map(|(width, _)| (width.0 as usize).max(MIN_TERMINAL_WIDTH))
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
}

macro_rules! monitor_handler {
    ($monitor:expr, $s:stmt) => {
        match $monitor {
            Some(monitor) => {
                ::std::thread::spawn(move || {
                    loop {
                        let key = ::getch::Getch::new().getch().unwrap();

                        if let b'q' = key {
                            break;
                        }
                    }

                    ::std::process::exit(0);
                });

                let sleep_interval = monitor;

                loop {
                    ::std::io::stdout().write_all(&crate::terminal::CLEAR_SCREEN_DATA).unwrap();

                    $s

                    ::std::thread::sleep(sleep_interval);
                }
            }
            None => {
                $s
            }
        }
    };
    ($monitor:expr, $monitor_interval_milli_secs:expr, $s:stmt) => {
        if $monitor {
            ::std::thread::spawn(move || {
                loop {
                    let key = ::getch::Getch::new().getch().unwrap();

                    if let b'q' = key {
                        break;
                    }
                }

                ::std::process::exit(0);
            });

            let sleep_interval = ::std::time::Duration::from_millis($monitor_interval_milli_secs);

            loop {
                ::std::io::stdout().write_all(&crate::terminal::CLEAR_SCREEN_DATA).unwrap();

                $s

                ::std::thread::sleep(sleep_interval);
            }
        } else {
            $s
        }
    };
    ($monitor:expr, $s:stmt, $si:stmt, $no_self_sleep:expr) => {
        match $monitor {
            Some(monitor) => {
                ::std::thread::spawn(move || {
                    loop {
                        let key = ::getch::Getch::new().getch().unwrap();

                        if let b'q' = key {
                            break;
                        }
                    }

                    ::std::process::exit(0);
                });

                ::std::io::stdout().write_all(&CLEAR_SCREEN_DATA).unwrap();

                $si

                let sleep_interval = monitor;

                loop {
                    if $no_self_sleep {
                        ::std::thread::sleep(sleep_interval);
                    }

                    ::std::io::stdout().write_all(&CLEAR_SCREEN_DATA).unwrap();

                    $s
                }
            }
            None => {
                $s
            }
        }
    };
}

pub(crate) use monitor_handler;
