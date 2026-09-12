use std::{
    env,
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
};
pub use std::{io::Write, time::Duration};

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

// `set_color_mode` runs before any of the color specs below is initialized, but a `LazyLock` may be initialized on any thread, so these have to be synchronized.
static FORCE_PLAIN_MODE: AtomicBool = AtomicBool::new(false);
static LIGHT_MODE: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn is_plain_mode() -> bool {
    FORCE_PLAIN_MODE.load(Ordering::Relaxed)
}

#[inline]
fn is_light_mode() -> bool {
    LIGHT_MODE.load(Ordering::Relaxed)
}

pub static COLOR_DEFAULT: LazyLock<ColorSpec> = LazyLock::new(ColorSpec::new);

pub static COLOR_LABEL: LazyLock<ColorSpec> = LazyLock::new(|| {
    let mut color_spec = ColorSpec::new();

    if !is_plain_mode() {
        if is_light_mode() {
            color_spec.set_fg(Some(DARK_CYAN_COLOR));
        } else {
            color_spec.set_fg(Some(CYAN_COLOR));
        }
    }

    color_spec
});

pub static COLOR_NORMAL_TEXT: LazyLock<ColorSpec> = LazyLock::new(|| {
    let mut color_spec = ColorSpec::new();

    if !is_plain_mode() {
        if is_light_mode() {
            color_spec.set_fg(Some(BLACK_COLOR));
        } else {
            color_spec.set_fg(Some(WHITE_COLOR));
        }
    }

    color_spec
});

pub static COLOR_BOLD_TEXT: LazyLock<ColorSpec> = LazyLock::new(|| {
    let mut color_spec = ColorSpec::new();

    if !is_plain_mode() {
        if is_light_mode() {
            color_spec.set_fg(Some(BLACK_COLOR)).set_bold(true);
        } else {
            color_spec.set_fg(Some(WHITE_COLOR)).set_bold(true);
        }
    }

    color_spec
});

pub static COLOR_USED: LazyLock<ColorSpec> = LazyLock::new(|| {
    let mut color_spec = ColorSpec::new();

    if !is_plain_mode() {
        if is_light_mode() {
            color_spec.set_fg(Some(WINE_COLOR));
        } else {
            color_spec.set_fg(Some(RED_COLOR));
        }
    }

    color_spec
});

pub static COLOR_CACHE: LazyLock<ColorSpec> = LazyLock::new(|| {
    let mut color_spec = ColorSpec::new();

    if !is_plain_mode() {
        if is_light_mode() {
            color_spec.set_fg(Some(ORANGE_COLOR));
        } else {
            color_spec.set_fg(Some(YELLOW_COLOR));
        }
    }

    color_spec
});

pub static COLOR_BUFFERS: LazyLock<ColorSpec> = LazyLock::new(|| {
    let mut color_spec = ColorSpec::new();

    if !is_plain_mode() {
        if is_light_mode() {
            color_spec.set_fg(Some(DARK_BLUE_COLOR));
        } else {
            color_spec.set_fg(Some(SKY_CYAN_COLOR));
        }
    }

    color_spec
});

pub fn set_color_mode(plain: bool, light: bool) {
    if plain {
        FORCE_PLAIN_MODE.store(true, Ordering::Relaxed);
    } else {
        match env::var_os(ENV_FORCE_PLAIN).map(|v| v.ne("0")) {
            Some(true) => {
                FORCE_PLAIN_MODE.store(true, Ordering::Relaxed);
            },
            _ => {
                let light =
                    light || env::var_os(ENV_LIGHT_MODE).map(|v| v.ne("0")).unwrap_or(false);

                LIGHT_MODE.store(light, Ordering::Relaxed);
            },
        }
    }
}

pub fn get_stdout_output() -> BufferWriter {
    if is_plain_mode() {
        BufferWriter::stdout(ColorChoice::Never)
    } else {
        BufferWriter::stdout(ColorChoice::Always)
    }
}

/// A total of zero means the resource is absent, e.g. a machine with no swap or a file system which reports no size, so it reads as empty rather than as `NaN`.
#[inline]
pub fn percentage_of(used: u64, total: u64) -> f64 {
    if total == 0 { 0f64 } else { used as f64 * 100f64 / total as f64 }
}

/// How many cells of a bar `value` fills, taken out of what the earlier segments left.
///
/// The memory `used` is `total - available`, and `available` counts part of the cache as free, so
/// the segments can add up to more than `total` on a machine under memory pressure. Drawing them out
/// of a shared remainder keeps the bar from running past its end, which would otherwise underflow
/// the count of the blank cells that follow.
#[inline]
pub fn bar_cells(value: u64, total: u64, progress_max: usize, remaining: &mut usize) -> usize {
    if total == 0 {
        return 0;
    }

    let cells =
        ((value as f64 * progress_max as f64 / total as f64).floor() as usize).min(*remaining);

    *remaining -= cells;

    cells
}

/// The columns can be wider than the terminal, and what is left over must not wrap around to a huge count of cells.
#[inline]
pub fn bar_width(terminal_width: usize, occupied: usize) -> usize {
    terminal_width.saturating_sub(occupied).max(1)
}

/// A bar cell and the padding around it are the same byte written many times, which `write!` cannot do without allocating a string.
pub fn write_cells(output: &mut impl Write, cell: u8, count: usize) -> std::io::Result<()> {
    const CHUNK: usize = 64;

    let buffer = [cell; CHUNK];

    let mut remaining = count;

    while remaining > CHUNK {
        output.write_all(&buffer)?;

        remaining -= CHUNK;
    }

    output.write_all(&buffer[..remaining])
}

pub fn get_term_width() -> usize {
    terminal_size()
        .map(|(width, _)| (width.0 as usize).max(MIN_TERMINAL_WIDTH))
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
}

/// Watch for the `q` that stops a monitoring loop, on a thread of its own since reading a key blocks.
pub fn spawn_quit_watcher() {
    ::std::thread::spawn(|| {
        // `Getch` puts the terminal back the way it found it when it is dropped, and `exit` runs no destructors, so it has to go out of scope first.
        {
            let getch = ::getch::Getch::new();

            loop {
                match getch.getch() {
                    Ok(b'q') => break,
                    // Reading nothing means stdin is at its end, e.g. it was redirected from `/dev/null`, so no key will ever arrive and only a signal can stop this run.
                    Ok(0) | Err(_) => return,
                    Ok(_) => (),
                }
            }
        }

        ::std::process::exit(0);
    });
}

macro_rules! monitor_handler {
    ($monitor:expr, $s:stmt) => {
        match $monitor {
            Some(monitor) => {
                crate::terminal::spawn_quit_watcher();

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
    ($monitor:expr, $s:stmt, $si:stmt, $no_self_sleep:expr) => {
        match $monitor {
            Some(monitor) => {
                crate::terminal::spawn_quit_watcher();

                ::std::io::stdout().write_all(&crate::terminal::CLEAR_SCREEN_DATA).unwrap();

                $si

                let sleep_interval = monitor;

                loop {
                    if $no_self_sleep {
                        ::std::thread::sleep(sleep_interval);
                    }

                    ::std::io::stdout().write_all(&crate::terminal::CLEAR_SCREEN_DATA).unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_of_an_absent_resource() {
        assert_eq!(0f64, percentage_of(0, 0));
        assert_eq!(50f64, percentage_of(1, 2));
    }

    #[test]
    fn test_bar_cells_of_an_absent_resource() {
        let mut remaining = 40;

        assert_eq!(0, bar_cells(0, 0, 40, &mut remaining));
        assert_eq!(40, remaining);
    }

    #[test]
    fn test_bar_width_of_a_terminal_narrower_than_the_columns() {
        assert_eq!(40, bar_width(80, 40));
        assert_eq!(1, bar_width(80, 80));
        assert_eq!(1, bar_width(80, 200));
    }

    #[test]
    fn test_write_cells() {
        let mut buffer = Vec::new();

        write_cells(&mut buffer, b'|', 0).unwrap();
        assert_eq!(b"", buffer.as_slice());

        write_cells(&mut buffer, b'|', 3).unwrap();
        assert_eq!(b"|||", buffer.as_slice());

        // More than one chunk.
        let mut buffer = Vec::new();

        write_cells(&mut buffer, b' ', 100).unwrap();
        assert_eq!(100, buffer.len());
        assert!(buffer.iter().all(|byte| *byte == b' '));
    }

    #[test]
    fn test_bar_cells_never_run_past_the_bar() {
        let mut remaining = 40;

        // `used + cache` can exceed the total on a machine under memory pressure.
        assert_eq!(32, bar_cells(80, 100, 40, &mut remaining));
        assert_eq!(8, bar_cells(60, 100, 40, &mut remaining));
        assert_eq!(0, bar_cells(60, 100, 40, &mut remaining));
        assert_eq!(0, remaining);
    }
}
