use std::{borrow::Cow, cmp::Ordering};

use anyhow::anyhow;
use byte_unit::{Byte, Unit, UnitType};
use chrono::SecondsFormat;
use mprober_lib::process;
use regex::Regex;
use terminal_size::terminal_size;
use unicode_width::UnicodeWidthChar;
use uzers::{Groups, Users, UsersCache};

use crate::{CLIArgs, CLICommands, terminal::*};

/// One process, formatted. Holding the columns of a row together keeps them from drifting apart when a row is cut short because the terminal is too narrow.
struct Row {
    pid:        String,
    ppid:       String,
    /// The real-time priority is marked with a `*`, so this is not a number.
    priority:   String,
    nice:       String,
    percentage: f64,
    vsz:        String,
    rss:        String,
    anon:       String,
    thd:        String,
    tty:        String,
    user:       String,
    group:      String,
    program:    String,
    state:      &'static str,
    /// Empty unless the start time was asked for.
    start_time: String,
    cmdline:    String,
}

/// The width of a name column: wide enough for its heading, and never wider than the truncation length, which can be shorter than that heading.
fn column_width(widths: impl Iterator<Item = usize>, heading: usize, truncate_inc: usize) -> usize {
    widths.max().unwrap_or(0).min(truncate_inc).max(heading)
}

/// Cut a name that is wider than `width` columns down to it and mark it with a trailing `+`.
///
/// A character outside ASCII can be two columns wide and is several bytes long, so neither count stands in for the other here.
fn truncate_with_marker(name: &str, width: usize) -> Cow<'_, str> {
    if display_width(name) <= width {
        return Cow::Borrowed(name);
    }

    // One column goes to the marker.
    let Some(budget) = width.checked_sub(1) else {
        return Cow::Borrowed("");
    };

    let mut taken = 0;
    let mut end = 0;

    for (index, character) in name.char_indices() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);

        if taken + character_width > budget {
            break;
        }

        taken += character_width;
        end = index + character.len_utf8();
    }

    Cow::Owned(format!("{}+", &name[..end]))
}

/// Below this a process is idle enough that its memory size says more about it than its share of the CPU does.
const BUSY_PERCENTAGE: f64 = 0.01;

/// Order the busy processes first, the busiest of them at the top, and the idle ones after them by memory size.
///
/// Two processes can be handed exactly the same number of jiffies over one interval, so the busy ones have to fall back to the same key as the idle ones rather than calling either of them the greater.
fn busier_first(a: (f64, u64), b: (f64, u64)) -> Ordering {
    let (percentage_a, vsz_a) = a;
    let (percentage_b, vsz_b) = b;

    match (percentage_a > BUSY_PERCENTAGE, percentage_b > BUSY_PERCENTAGE) {
        (true, true) => percentage_b.total_cmp(&percentage_a).then(vsz_b.cmp(&vsz_a)),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => vsz_b.cmp(&vsz_a),
    }
}

#[inline]
pub fn handle_process(args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(args.command, CLICommands::Process { .. }));

    if let CLICommands::Process {
        plain,
        light,
        monitor,
        unit,
        only_information,
        top,
        truncate,
        start_time,
        user_filter,
        group_filter,
        program_filter,
        tty_filter,
        pid_filter,
    } = args.command
    {
        let user_filter = user_filter.as_deref();
        let group_filter = group_filter.as_deref();
        let program_filter = program_filter.as_ref();
        let tty_filter = tty_filter.as_ref();

        set_color_mode(plain, light);

        monitor_handler!(
            monitor,
            draw_process(
                monitor,
                top,
                truncate,
                unit,
                only_information,
                start_time,
                user_filter,
                group_filter,
                program_filter,
                tty_filter,
                pid_filter,
            )?,
            draw_process(
                Some(DEFAULT_INTERVAL),
                top,
                truncate,
                unit,
                only_information,
                start_time,
                user_filter,
                group_filter,
                program_filter,
                tty_filter,
                pid_filter,
            )?,
            only_information
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_process(
    monitor: Option<Duration>,
    mut top: Option<usize>,
    truncate: usize,
    unit: Option<Unit>,
    only_information: bool,
    start_time: bool,
    user_filter: Option<&str>,
    group_filter: Option<&str>,
    program_filter: Option<&Regex>,
    tty_filter: Option<&Regex>,
    pid_filter: Option<u32>,
) -> anyhow::Result<()> {
    let output = get_stdout_output();
    let mut stdout = output.buffer();

    let terminal_width = match terminal_size() {
        Some((width, height)) => {
            if monitor.is_some() {
                let height = (height.0 as usize).max(2) - 2;

                top = match top {
                    Some(top) => Some(top.min(height)),
                    None => Some(height),
                };
            }

            (width.0 as usize).max(MIN_TERMINAL_WIDTH)
        },
        None => DEFAULT_TERMINAL_WIDTH,
    };

    let user_cache = UsersCache::new();

    let uid_filter = match user_filter {
        Some(user_filter) => match user_cache.get_user_by_name(user_filter) {
            Some(user) => Some(user.uid()),
            None => {
                return Err(anyhow!("Cannot find the user {:?}.", user_filter));
            },
        },
        None => None,
    };

    let gid_filter = match group_filter {
        Some(group_filter) => match user_cache.get_group_by_name(group_filter) {
            Some(group) => Some(group.gid()),
            None => {
                return Err(anyhow!("Cannot find the group {:?}.", group_filter));
            },
        },
        None => None,
    };

    // The filters are predicates now, so the regexes have to be wrapped before they are borrowed.
    let program_matcher = program_filter.map(|regex| move |program: &str| regex.is_match(program));
    let tty_matcher = tty_filter.map(|regex| move |tty: &str| regex.is_match(tty));

    let process_filter = process::ProcessFilter {
        pid_filter,
        uid_filter,
        gid_filter,
        program_filter: program_matcher.as_ref().map(|matcher| matcher as &dyn Fn(&str) -> bool),
        tty_filter: tty_matcher.as_ref().map(|matcher| matcher as &dyn Fn(&str) -> bool),
    };

    let processes: Vec<(process::Process, f64)> = if only_information {
        let mut processes_with_stats = process::get_processes_with_stat(&process_filter).unwrap();

        processes_with_stats.sort_unstable_by_key(|(a, _)| std::cmp::Reverse(a.vsz));

        if let Some(top) = top {
            processes_with_stats.truncate(top);
        }

        processes_with_stats.into_iter().map(|(process, _)| (process, 0f64)).collect()
    } else {
        let mut processes_with_percentage =
            process::get_processes_with_cpu_utilization_in_percentage(
                &process_filter,
                monitor.unwrap_or(DEFAULT_INTERVAL),
            )
            .unwrap();

        processes_with_percentage.sort_unstable_by(
            |(process_a, percentage_a), (process_b, percentage_b)| {
                busier_first((*percentage_a, process_a.vsz), (*percentage_b, process_b.vsz))
            },
        );

        if let Some(top) = top {
            processes_with_percentage.truncate(top);
        }

        processes_with_percentage
    };

    let format_byte = |value: u64| match unit {
        Some(unit) => format!("{:.1}", Byte::from(value).get_adjusted_unit(unit)),
        None => format!("{:.1}", Byte::from(value).get_appropriate_unit(UnitType::Binary)),
    };

    let rows: Vec<Row> = processes
        .into_iter()
        .map(|(process, percentage)| Row {
            pid: process.pid.to_string(),
            ppid: process.ppid.to_string(),
            priority: match process.real_time_priority {
                Some(real_time_priority) => format!("*{real_time_priority}"),
                None => process.priority.to_string(),
            },
            nice: process.nice.to_string(),
            percentage,
            vsz: format_byte(process.vsz),
            rss: format_byte(process.rss),
            anon: format_byte(process.rss_anon),
            thd: process.threads.to_string(),
            tty: process.tty.unwrap_or_default(),
            // TODO: musl cannot directly handle dynamic users (with systemd). It causes `UserCache` returns `None`.
            user: user_cache
                .get_user_by_uid(process.effective_uid)
                .map(|user| user.name().to_string_lossy().into_owned())
                .unwrap_or_else(|| String::from("systemd?")),
            group: user_cache
                .get_group_by_gid(process.effective_gid)
                .map(|group| group.name().to_string_lossy().into_owned())
                .unwrap_or_else(|| String::from("systemd?")),
            program: process.program,
            state: process.state.as_str(),
            start_time: if start_time {
                process.start_time.to_rfc3339_opts(SecondsFormat::Secs, true)
            } else {
                String::new()
            },
            cmdline: process.cmdline,
        })
        .collect();

    let truncate_inc = if truncate == 0 { usize::MAX } else { truncate + 1 };

    let pid_len = rows.iter().map(|row| row.pid.len()).max().map(|s| s.max(5)).unwrap_or(0);
    let ppid_len = rows.iter().map(|row| row.ppid.len()).max().map(|s| s.max(5)).unwrap_or(0);
    let vsz_len = rows.iter().map(|row| row.vsz.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let rss_len = rows.iter().map(|row| row.rss.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let anon_len = rows.iter().map(|row| row.anon.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let thd_len = rows.iter().map(|row| row.thd.len()).max().map(|s| s.max(3)).unwrap_or(0);
    let tty_len =
        rows.iter().map(|row| display_width(&row.tty)).max().map(|s| s.max(4)).unwrap_or(0);
    let state_len = rows.iter().map(|row| row.state.len()).max().map(|s| s.max(5)).unwrap_or(0);

    let user_len = column_width(rows.iter().map(|row| display_width(&row.user)), 4, truncate_inc);
    let group_len = column_width(rows.iter().map(|row| display_width(&row.group)), 5, truncate_inc);
    let program_len =
        column_width(rows.iter().map(|row| display_width(&row.program)), 7, truncate_inc);

    'header: {
        let mut width = 0;

        stdout.set_color(&COLOR_LABEL).unwrap();

        if width + pid_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "{:1$}", "", pid_len.saturating_sub(3)).unwrap();
        width += pid_len.saturating_sub(3);

        write!(&mut stdout, "PID").unwrap(); // 3
        width += 3;

        if width + 1 + ppid_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "{:1$}", "", ppid_len.saturating_sub(3)).unwrap();
        width += ppid_len.saturating_sub(3);

        write!(&mut stdout, "PPID").unwrap(); // 4
        width += 4;

        if width + 5 > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "   PR").unwrap(); // 5
        width += 5;

        if width + 4 > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "  NI").unwrap(); // 4
        width += 4;

        if !only_information {
            if width + 5 > terminal_width {
                break 'header;
            }

            write!(&mut stdout, " %CPU").unwrap(); // 5
            width += 5;
        }

        if width + 1 + vsz_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "{:1$}", "", vsz_len.saturating_sub(2)).unwrap();
        width += vsz_len.saturating_sub(2);

        write!(&mut stdout, "VSZ").unwrap(); // 3
        width += 3;

        if width + 1 + rss_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "{:1$}", "", rss_len.saturating_sub(2)).unwrap();
        width += rss_len.saturating_sub(2);

        write!(&mut stdout, "RSS").unwrap(); // 3
        width += 3;

        if width + 1 + anon_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "{:1$}", "", anon_len.saturating_sub(3)).unwrap();
        width += anon_len.saturating_sub(3);

        write!(&mut stdout, "ANON").unwrap(); // 4
        width += 4;

        if width + 1 + thd_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, "{:1$}", "", thd_len.saturating_sub(2)).unwrap();
        width += thd_len.saturating_sub(2);

        write!(&mut stdout, "THD").unwrap(); // 3
        width += 3;

        if width + 1 + tty_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, " TTY").unwrap(); // 4
        width += 4;

        write!(&mut stdout, "{:1$}", "", tty_len.saturating_sub(3)).unwrap();
        width += tty_len.saturating_sub(3);

        if width + 1 + user_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, " USER").unwrap(); // 5
        width += 5;

        write!(&mut stdout, "{:1$}", "", user_len.saturating_sub(4)).unwrap();
        width += user_len.saturating_sub(4);

        if width + 1 + group_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, " GROUP").unwrap(); // 6
        width += 6;

        write!(&mut stdout, "{:1$}", "", group_len.saturating_sub(5)).unwrap();
        width += group_len.saturating_sub(5);

        if width + 1 + program_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, " PROGRAM").unwrap(); // 8
        width += 8;

        write!(&mut stdout, "{:1$}", "", program_len.saturating_sub(7)).unwrap();
        width += program_len.saturating_sub(7);

        if width + 1 + state_len > terminal_width {
            break 'header;
        }

        write!(&mut stdout, " STATE").unwrap(); // 6
        width += 6;

        write!(&mut stdout, "{:1$}", "", state_len.saturating_sub(5)).unwrap();
        width += state_len.saturating_sub(5);

        if start_time {
            if width + 21 > terminal_width {
                break 'header;
            }

            write!(&mut stdout, " START").unwrap(); // 6
            width += 6;

            write!(&mut stdout, "{:15}", "").unwrap();
            width += 15;
        }

        if width + 8 > terminal_width {
            break 'header;
        }

        write!(&mut stdout, " COMMAND").unwrap(); // 8
    }

    stdout.set_color(&COLOR_DEFAULT).unwrap();
    writeln!(&mut stdout).unwrap();

    for row in rows.iter() {
        let mut width = 0;

        if width + pid_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        write!(&mut stdout, "{1:>0$}", pid_len, row.pid).unwrap();
        width += pid_len;

        if width + 1 + ppid_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();

        write!(&mut stdout, "{1:>0$}", ppid_len + 1, row.ppid).unwrap();
        width += ppid_len + 1;

        if width + 5 > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, "{:>5}", row.priority).unwrap();
        width += 5;

        if width + 4 > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, "{:>4}", row.nice).unwrap();
        width += 4;

        if !only_information {
            if width + 5 > terminal_width {
                stdout.set_color(&COLOR_DEFAULT).unwrap();
                writeln!(&mut stdout).unwrap();

                continue;
            }

            write!(&mut stdout, " {:>4.1}", row.percentage * 100.0).unwrap();
            width += 5;
        }

        if width + 1 + vsz_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, "{1:>0$}", vsz_len + 1, row.vsz).unwrap();
        width += vsz_len + 1;

        if width + 1 + rss_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, "{1:>0$}", rss_len + 1, row.rss).unwrap();
        width += rss_len + 1;

        if width + 1 + anon_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, "{1:>0$}", anon_len + 1, row.anon).unwrap();
        width += anon_len + 1;

        if width + 1 + thd_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, "{1:>0$}", thd_len + 1, row.thd).unwrap();
        width += thd_len + 1;

        if width + 1 + tty_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, " ").unwrap();
        write_left_aligned(&mut stdout, &row.tty, tty_len).unwrap();
        width += 1 + tty_len;

        if width + 1 + user_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, " ").unwrap();
        write_left_aligned(&mut stdout, &truncate_with_marker(&row.user, truncate_inc), user_len)
            .unwrap();
        width += 1 + user_len;

        if width + 1 + group_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, " ").unwrap();
        write_left_aligned(&mut stdout, &truncate_with_marker(&row.group, truncate_inc), group_len)
            .unwrap();
        width += 1 + group_len;

        if width + 1 + program_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, " ").unwrap();
        write_left_aligned(
            &mut stdout,
            &truncate_with_marker(&row.program, truncate_inc),
            program_len,
        )
        .unwrap();
        width += 1 + program_len;

        if width + 1 + state_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, " {1:<0$}", state_len, row.state).unwrap();
        width += 1 + state_len;

        if start_time {
            if width + 21 > terminal_width {
                stdout.set_color(&COLOR_DEFAULT).unwrap();
                writeln!(&mut stdout).unwrap();

                continue;
            }

            write!(&mut stdout, " ").unwrap(); // 1

            stdout.write_all(row.start_time.as_bytes()).unwrap();

            width += 21;
        }

        if width + 8 > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, " ").unwrap(); // 1
        width += 1;

        let remain_width = terminal_width - width;

        stdout.write_all(truncate_with_marker(&row.cmdline, remain_width).as_bytes()).unwrap();

        stdout.set_color(&COLOR_DEFAULT).unwrap();
        writeln!(&mut stdout).unwrap();
    }

    output.print(&stdout).unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_width_of_a_truncation_shorter_than_the_heading() {
        // `--truncate 5` leaves 6 for a name, which is narrower than the 7 of `PROGRAM`.
        assert_eq!(7, column_width([8, 12].into_iter(), 7, 6));
        assert_eq!(4, column_width([8, 12].into_iter(), 4, 2));
    }

    #[test]
    fn test_column_width_without_a_truncation() {
        // `--truncate 0` turns the truncation off.
        assert_eq!(12, column_width([8, 12].into_iter(), 7, usize::MAX));
        assert_eq!(7, column_width([3, 5].into_iter(), 7, usize::MAX));
    }

    #[test]
    fn test_column_width_of_an_empty_list() {
        // Nothing matched the filters, so only the heading has to fit.
        assert_eq!(7, column_width(std::iter::empty(), 7, usize::MAX));
    }

    #[test]
    fn test_truncate_with_marker() {
        assert_eq!("magiclen", truncate_with_marker("magiclen", 8));
        assert_eq!("magicl+", truncate_with_marker("magiclen", 7));
    }

    #[test]
    fn test_busier_first_puts_the_busy_processes_on_top() {
        assert_eq!(Ordering::Less, busier_first((0.50, 1), (0.20, 4096)));
        assert_eq!(Ordering::Greater, busier_first((0.20, 4096), (0.50, 1)));

        // An idle process is ordered by its memory size, however much the other one has.
        assert_eq!(Ordering::Less, busier_first((0.001, 4096), (0.001, 1)));
    }

    #[test]
    fn test_busier_first_of_an_equal_percentage() {
        assert_eq!(Ordering::Less, busier_first((0.5, 4096), (0.5, 1)));
        assert_eq!(Ordering::Greater, busier_first((0.5, 1), (0.5, 4096)));
        assert_eq!(Ordering::Equal, busier_first((0.5, 4096), (0.5, 4096)));
    }

    #[test]
    fn test_truncate_with_marker_counts_columns_not_bytes() {
        // Three characters, six columns, nine bytes, so a six-column field holds it whole.
        assert_eq!("日本語", truncate_with_marker("日本語", 6));
        assert_eq!("日本+", truncate_with_marker("日本語", 5));
        assert_eq!("日+", truncate_with_marker("日本語", 3));

        // Not even one character fits beside the marker.
        assert_eq!("+", truncate_with_marker("日本語", 2));
    }
}
