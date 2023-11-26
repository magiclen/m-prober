use std::{cmp::Ordering, collections::BTreeMap, sync::Arc};

use anyhow::anyhow;
use byte_unit::{Byte, Unit, UnitType};
use chrono::SecondsFormat;
use mprober_lib::process;
use regex::Regex;
use terminal_size::terminal_size;
use users::{Group, Groups, User, Users, UsersCache};

use crate::{terminal::*, CLIArgs, CLICommands};

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

#[allow(unused_variables)]
#[allow(unused_mut)]
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
                return Err(anyhow!("Cannot find the user {:?}.", group_filter));
            },
        },
        None => None,
    };

    let process_filter = process::ProcessFilter {
        pid_filter,
        uid_filter,
        gid_filter,
        program_filter,
        tty_filter,
    };

    let (processes, percentage): (Vec<process::Process>, BTreeMap<u32, f64>) = if only_information {
        let mut processes_with_stats = process::get_processes_with_stat(&process_filter).unwrap();

        processes_with_stats.sort_unstable_by(|(a, _), (b, _)| b.vsz.cmp(&a.vsz));

        if let Some(top) = top {
            if top < processes_with_stats.len() {
                unsafe {
                    processes_with_stats.set_len(top);
                }
            }
        }

        (processes_with_stats.into_iter().map(|(process, _)| process).collect(), BTreeMap::new())
    } else {
        let mut processes_with_percentage =
            process::get_processes_with_cpu_utilization_in_percentage(
                &process_filter,
                match monitor {
                    Some(monitor) => monitor,
                    None => DEFAULT_INTERVAL,
                },
            )
            .unwrap();

        processes_with_percentage.sort_unstable_by(
            |(process_a, percentage_a), (process_b, percentage_b)| {
                let percentage_a = *percentage_a;
                let percentage_b = *percentage_b;

                if percentage_a > 0.01 {
                    if percentage_a > percentage_b {
                        Ordering::Less
                    } else if percentage_b > 0.01
                    // percentage_a == percentage_b hardly happens
                    {
                        Ordering::Greater
                    } else {
                        process_b.vsz.cmp(&process_a.vsz)
                    }
                } else if percentage_b > 0.01 {
                    if percentage_b > percentage_a {
                        Ordering::Greater
                    } else {
                        process_b.vsz.cmp(&process_a.vsz)
                    }
                } else {
                    process_b.vsz.cmp(&process_a.vsz)
                }
            },
        );

        if let Some(top) = top {
            if top < processes_with_percentage.len() {
                unsafe {
                    processes_with_percentage.set_len(top);
                }
            }
        }

        let mut processes = Vec::with_capacity(processes_with_percentage.len());
        let mut processes_percentage = BTreeMap::new();

        for (process, percentage) in processes_with_percentage {
            processes_percentage.insert(process.pid, percentage);

            processes.push(process);
        }

        (processes, processes_percentage)
    };

    let processes_len = processes.len();

    let mut pid: Vec<String> = Vec::with_capacity(processes_len);
    let mut ppid: Vec<String> = Vec::with_capacity(processes_len);
    let mut vsz: Vec<String> = Vec::with_capacity(processes_len);
    let mut rss: Vec<String> = Vec::with_capacity(processes_len);
    let mut anon: Vec<String> = Vec::with_capacity(processes_len);
    let mut thd: Vec<String> = Vec::with_capacity(processes_len);
    let mut tty: Vec<&str> = Vec::with_capacity(processes_len);
    let mut user: Vec<Arc<User>> = Vec::with_capacity(processes_len);
    let mut group: Vec<Arc<Group>> = Vec::with_capacity(processes_len);
    let mut program: Vec<&str> = Vec::with_capacity(processes_len);
    let mut state: Vec<&'static str> = Vec::with_capacity(processes_len);

    for process in processes.iter() {
        pid.push(process.pid.to_string());
        ppid.push(process.ppid.to_string());

        let (p_vsz, p_rss, p_anon) =
            (Byte::from(process.vsz), Byte::from(process.rss), Byte::from(process.rss_anon));

        match unit {
            Some(byte_unit) => {
                vsz.push(format!("{:.1}", p_vsz.get_adjusted_unit(byte_unit)));
                rss.push(format!("{:.1}", p_rss.get_adjusted_unit(byte_unit)));
                anon.push(format!("{:.1}", p_anon.get_adjusted_unit(byte_unit)));
            },
            None => {
                vsz.push(format!("{:.1}", p_vsz.get_appropriate_unit(UnitType::Binary)));
                rss.push(format!("{:.1}", p_rss.get_appropriate_unit(UnitType::Binary)));
                anon.push(format!("{:.1}", p_anon.get_appropriate_unit(UnitType::Binary)));
            },
        }

        tty.push(process.tty.as_deref().unwrap_or(""));

        thd.push(process.threads.to_string());

        // TODO: musl cannot directly handle dynamic users (with systemd). It causes `UserCache` returns `None`.
        user.push(
            user_cache
                .get_user_by_uid(process.effective_uid)
                .unwrap_or_else(|| Arc::new(User::new(0, "systemd?", 0))),
        );
        group.push(
            user_cache
                .get_group_by_gid(process.effective_gid)
                .unwrap_or_else(|| Arc::new(Group::new(0, "systemd?"))),
        );

        program.push(process.program.as_str());
        state.push(process.state.as_str());
    }

    let truncate_inc = if truncate == 0 { usize::MAX } else { truncate + 1 };

    let pid_len = pid.iter().map(|s| s.len()).max().map(|s| s.max(5)).unwrap_or(0);
    let ppid_len = ppid.iter().map(|s| s.len()).max().map(|s| s.max(5)).unwrap_or(0);
    let vsz_len = vsz.iter().map(|s| s.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let rss_len = rss.iter().map(|s| s.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let anon_len = anon.iter().map(|s| s.len()).max().map(|s| s.max(9)).unwrap_or(0);
    let thd_len = thd.iter().map(|s| s.len()).max().map(|s| s.max(3)).unwrap_or(0);
    let tty_len = tty.iter().map(|s| s.len()).max().map(|s| s.max(4)).unwrap_or(0);
    let user_len = user
        .iter()
        .map(|user| user.name().len())
        .max()
        .map(|s| s.clamp(4, truncate_inc))
        .unwrap_or(truncate_inc);
    let group_len = group
        .iter()
        .map(|group| group.name().len())
        .max()
        .map(|s| s.clamp(5, truncate_inc))
        .unwrap_or(truncate_inc);
    let program_len = program
        .iter()
        .map(|s| s.len())
        .max()
        .map(|s| s.clamp(7, truncate_inc))
        .unwrap_or(truncate_inc);
    let state_len = state.iter().map(|s| s.len()).max().map(|s| s.max(5)).unwrap_or(0);

    #[allow(clippy::never_loop)]
    loop {
        let mut width = 0;

        stdout.set_color(&COLOR_LABEL).unwrap();

        if width + pid_len > terminal_width {
            break;
        }

        for _ in 3..pid_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        write!(&mut stdout, "PID").unwrap(); // 3
        width += 3;

        if width + 1 + ppid_len > terminal_width {
            break;
        }

        for _ in 3..ppid_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        write!(&mut stdout, "PPID").unwrap(); // 4
        width += 4;

        if width + 5 > terminal_width {
            break;
        }

        write!(&mut stdout, "   PR").unwrap(); // 5
        width += 5;

        if width + 4 > terminal_width {
            break;
        }

        write!(&mut stdout, "  NI").unwrap(); // 4
        width += 4;

        if !only_information {
            if width + 5 > terminal_width {
                break;
            }

            write!(&mut stdout, " %CPU").unwrap(); // 5
            width += 5;
        }

        if width + 1 + vsz_len > terminal_width {
            break;
        }

        for _ in 2..vsz_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        write!(&mut stdout, "VSZ").unwrap(); // 3
        width += 3;

        if width + 1 + rss_len > terminal_width {
            break;
        }

        for _ in 2..rss_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        write!(&mut stdout, "RSS").unwrap(); // 3
        width += 3;

        if width + 1 + anon_len > terminal_width {
            break;
        }

        for _ in 3..anon_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        write!(&mut stdout, "ANON").unwrap(); // 4
        width += 4;

        if width + 1 + thd_len > terminal_width {
            break;
        }

        for _ in 2..thd_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        write!(&mut stdout, "THD").unwrap(); // 3
        width += 3;

        if width + 1 + tty_len > terminal_width {
            break;
        }

        write!(&mut stdout, " TTY").unwrap(); // 4
        width += 4;

        for _ in 3..tty_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        if width + 1 + user_len > terminal_width {
            break;
        }

        write!(&mut stdout, " USER").unwrap(); // 5
        width += 5;

        for _ in 4..user_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        if width + 1 + group_len > terminal_width {
            break;
        }

        write!(&mut stdout, " GROUP").unwrap(); // 6
        width += 6;

        for _ in 5..group_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        if width + 1 + program_len > terminal_width {
            break;
        }

        write!(&mut stdout, " PROGRAM").unwrap(); // 8
        width += 8;

        for _ in 7..program_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        if width + 1 + state_len > terminal_width {
            break;
        }

        write!(&mut stdout, " STATE").unwrap(); // 6
        width += 6;

        for _ in 5..state_len {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        if start_time {
            if width + 21 > terminal_width {
                break;
            }

            write!(&mut stdout, " START").unwrap(); // 6
            width += 6;

            for _ in 5..20 {
                write!(&mut stdout, " ").unwrap(); // 1
                width += 1;
            }
        }

        if width + 8 > terminal_width {
            break;
        }

        write!(&mut stdout, " COMMAND").unwrap(); // 8

        break;
    }

    stdout.set_color(&COLOR_DEFAULT).unwrap();
    writeln!(&mut stdout).unwrap();

    let mut pid_iter = pid.into_iter();
    let mut ppid_iter = ppid.into_iter();
    let mut vsz_iter = vsz.into_iter();
    let mut rss_iter = rss.into_iter();
    let mut tty_iter = tty.into_iter();
    let mut anon_iter = anon.into_iter();
    let mut thd_iter = thd.into_iter();
    let mut user_iter = user.into_iter();
    let mut group_iter = group.into_iter();
    let mut program_iter = program.into_iter();
    let mut state_iter = state.into_iter();

    for process in processes.iter() {
        let mut width = 0;

        if width + pid_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let pid = pid_iter.next().unwrap();

        stdout.set_color(&COLOR_BOLD_TEXT).unwrap();
        write!(&mut stdout, "{1:>0$}", pid_len, pid).unwrap();
        width += pid_len;

        if width + 1 + ppid_len > terminal_width {
            continue;
        }

        stdout.set_color(&COLOR_NORMAL_TEXT).unwrap();

        let ppid = ppid_iter.next().unwrap();

        for _ in 0..=(ppid_len - ppid.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        stdout.write_all(ppid.as_bytes()).unwrap();
        width += ppid.len();

        if width + 5 > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        if let Some(real_time_priority) = process.real_time_priority {
            write!(&mut stdout, "{:>5}", format!("*{}", real_time_priority)).unwrap();
        } else {
            write!(&mut stdout, "{:>5}", process.priority).unwrap();
        }
        width += 5;

        if width + 4 > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        write!(&mut stdout, "{:>4}", process.nice).unwrap();
        width += 4;

        if !only_information {
            if width + 5 > terminal_width {
                stdout.set_color(&COLOR_DEFAULT).unwrap();
                writeln!(&mut stdout).unwrap();

                continue;
            }

            write!(&mut stdout, " {:>4.1}", percentage.get(&process.pid).unwrap() * 100.0).unwrap();
            width += 5;
        }

        if width + 1 + vsz_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let vsz = vsz_iter.next().unwrap();

        for _ in 0..=(vsz_len - vsz.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        stdout.write_all(vsz.as_bytes()).unwrap();
        width += vsz.len();

        if width + 1 + rss_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let rss = rss_iter.next().unwrap();

        for _ in 0..=(rss_len - rss.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        stdout.write_all(rss.as_bytes()).unwrap();
        width += rss.len();

        if width + 1 + anon_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let anon = anon_iter.next().unwrap();

        for _ in 0..=(anon_len - anon.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        stdout.write_all(anon.as_bytes()).unwrap();
        width += anon.len();

        if width + 1 + thd_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let thd = thd_iter.next().unwrap();

        for _ in 0..=(thd_len - thd.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        stdout.write_all(thd.as_bytes()).unwrap();
        width += thd.len();

        if width + 1 + tty_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let tty = tty_iter.next().unwrap();

        write!(&mut stdout, " ").unwrap(); // 1
        width += 1;

        stdout.write_all(tty.as_bytes()).unwrap();
        width += tty.len();

        for _ in 0..(tty_len - tty.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        if width + 1 + user_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let user = user_iter.next().unwrap();

        write!(&mut stdout, " ").unwrap(); // 1
        width += 1;

        {
            let s = user.name().to_str().unwrap();

            if s.len() > truncate_inc {
                stdout.write_all(s[..(truncate_inc - 1)].as_bytes()).unwrap();
                write!(&mut stdout, "+").unwrap(); // 1
                width += truncate_inc;

                for _ in truncate_inc..4 {
                    write!(&mut stdout, " ").unwrap(); // 1
                    width += 1;
                }
            } else {
                stdout.write_all(s.as_bytes()).unwrap();
                width += s.len();

                for _ in 0..(user_len - s.len()) {
                    write!(&mut stdout, " ").unwrap(); // 1
                    width += 1;
                }
            }
        }

        if width + 1 + group_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let group = group_iter.next().unwrap();

        write!(&mut stdout, " ").unwrap(); // 1
        width += 1;

        {
            let s = group.name().to_str().unwrap();

            if s.len() > truncate_inc {
                stdout.write_all(s[..(truncate_inc - 1)].as_bytes()).unwrap();
                write!(&mut stdout, "+").unwrap(); // 1
                width += truncate_inc;

                for _ in truncate_inc..5 {
                    write!(&mut stdout, " ").unwrap(); // 1
                    width += 1;
                }
            } else {
                stdout.write_all(s.as_bytes()).unwrap();
                width += s.len();

                for _ in 0..(group_len - s.len()) {
                    write!(&mut stdout, " ").unwrap(); // 1
                    width += 1;
                }
            }
        }

        if width + 1 + program_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let program = program_iter.next().unwrap();

        write!(&mut stdout, " ").unwrap(); // 1
        width += 1;

        if program.len() > truncate_inc {
            stdout.write_all(program[..(truncate_inc - 1)].as_bytes()).unwrap();
            write!(&mut stdout, "+").unwrap(); // 1
            width += truncate_inc;

            for _ in truncate_inc..7 {
                write!(&mut stdout, " ").unwrap(); // 1
                width += 1;
            }
        } else {
            stdout.write_all(program.as_bytes()).unwrap();
            width += program.len();

            for _ in 0..(program_len - program.len()) {
                write!(&mut stdout, " ").unwrap(); // 1
                width += 1;
            }
        }

        if width + 1 + state_len > terminal_width {
            stdout.set_color(&COLOR_DEFAULT).unwrap();
            writeln!(&mut stdout).unwrap();

            continue;
        }

        let state = state_iter.next().unwrap();

        write!(&mut stdout, " ").unwrap(); // 1
        width += 1;

        stdout.write_all(state.as_bytes()).unwrap();
        width += state.len();

        for _ in 0..(state_len - state.len()) {
            write!(&mut stdout, " ").unwrap(); // 1
            width += 1;
        }

        if start_time {
            if width + 21 > terminal_width {
                stdout.set_color(&COLOR_DEFAULT).unwrap();
                writeln!(&mut stdout).unwrap();

                continue;
            }

            write!(&mut stdout, " ").unwrap(); // 1

            stdout
                .write_all(process.start_time.to_rfc3339_opts(SecondsFormat::Secs, true).as_bytes())
                .unwrap();

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

        if process.cmdline.len() > remain_width {
            let cmdline =
                String::from_utf8_lossy(&process.cmdline.as_bytes()[..(remain_width - 1)]);

            stdout.write_all(cmdline.as_bytes()).unwrap();
            write!(&mut stdout, "+").unwrap(); // 1
        } else {
            stdout.write_all(process.cmdline.as_bytes()).unwrap();
        }

        stdout.set_color(&COLOR_DEFAULT).unwrap();
        writeln!(&mut stdout).unwrap();
    }

    output.print(&stdout).unwrap();

    Ok(())
}
