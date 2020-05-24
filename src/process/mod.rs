use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, ErrorKind};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

mod state;

use crate::chrono::prelude::*;
use crate::scanner_rust::{Scanner, ScannerError};
use crate::time;
use crate::CPUStat;
use crate::Regex;

pub use state::ProcessState;

#[derive(Debug, Clone, Eq)]
pub struct Process {
    pub pid: u32,
    pub effective_uid: u32,
    pub effective_gid: u32,
    pub state: ProcessState,
    pub ppid: u32,
    pub cmdline: String,
    pub tty: Option<String>,
    pub priority: i8,
    pub nice: i8,
    pub threads: usize,
    /// Virtual Set Size (VIRT)
    pub vsz: usize,
    /// Resident Set Size (RES)
    pub rss: usize,
    pub start_time: DateTime<Utc>,
}

impl Hash for Process {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pid.hash(state)
    }
}

impl PartialEq for Process {
    #[inline]
    fn eq(&self, other: &Process) -> bool {
        self.pid.eq(&other.pid)
    }
}

impl Process {
    fn get_process_with_stat_inner<P: AsRef<Path>>(
        pid: u32,
        process_path: P,
        uid_filter: Option<u32>,
        program_filter: Option<&Regex>,
        tty_filter: Option<&Regex>,
    ) -> Result<Option<(Process, ProcessStat)>, ScannerError> {
        let process_path = process_path.as_ref();

        let mut program_filter_match = true;

        let status = ProcessStatus::get_process_status(pid)?;

        if let Some(uid_filter) = uid_filter {
            if status.real_uid != uid_filter
                && status.effective_uid != uid_filter
                && status.saved_set_uid != uid_filter
                && status.fs_uid != uid_filter
            {
                return Ok(None);
            }
        }

        let cmdline = fs::read_to_string(process_path.join("cmdline"))?;

        if let Some(program_filter) = program_filter {
            if !program_filter.is_match(&cmdline) {
                program_filter_match = false;
            }
        }

        let stat = ProcessStat::get_process_stat(pid)?;

        if !program_filter_match {
            if let Some(program_filter) = program_filter {
                if !program_filter.is_match(&stat.comm) {
                    return Ok(None);
                }
            }
        }

        let effective_uid = status.effective_uid;
        let effective_gid = status.effective_gid;
        let state = stat.state;
        let ppid = stat.ppid;

        let tty = {
            match stat.tty_nr_major {
                4 => {
                    if stat.tty_nr_minor < 64 {
                        Some(format!("tty{}", stat.tty_nr_minor))
                    } else {
                        Some(format!("ttyS{}", stat.tty_nr_minor - 64))
                    }
                }
                136..=143 => Some(format!("pts/{}", stat.tty_nr_minor)),
                _ => None,
            }
        };

        if let Some(tty_filter) = tty_filter {
            match tty.as_ref() {
                Some(tty) => {
                    if !tty_filter.is_match(tty) {
                        return Ok(None);
                    }
                }
                None => return Ok(None),
            }
        }

        let priority = stat.priority;
        let nice = stat.nice;
        let threads = stat.num_threads;
        let vsz = stat.vsize;
        let rss = stat.rss;

        let start_time = time::get_btime()?
            + chrono::Duration::from_std(Duration::from_millis(stat.starttime)).unwrap();

        let process = Process {
            pid,
            effective_uid,
            effective_gid,
            state,
            ppid,
            cmdline,
            tty,
            priority,
            nice,
            threads,
            start_time,
            vsz,
            rss,
        };

        Ok(Some((process, stat)))
    }

    #[inline]
    pub fn get_process_with_stat(pid: u32) -> Result<(Process, ProcessStat), ScannerError> {
        let process_path = Path::new("/proc").join(pid.to_string());

        Process::get_process_with_stat_inner(pid, process_path, None, None, None)
            .map(|o| o.unwrap())
    }

    pub fn get_processes_with_stats(
        uid_filter: Option<u32>,
        program_filter: Option<&Regex>,
        tty_filter: Option<&Regex>,
    ) -> Result<Vec<(Process, ProcessStat)>, ScannerError> {
        let mut processes_with_stats = Vec::new();

        let proc = Path::new("/proc");

        for dir_entry in proc.read_dir()? {
            let dir_entry = dir_entry?;

            if let Some(file_name) = dir_entry.file_name().to_str() {
                if let Ok(pid) = file_name.parse::<u32>() {
                    let process_path = dir_entry.path();

                    if let Some((process, stat)) = Process::get_process_with_stat_inner(
                        pid,
                        process_path,
                        uid_filter,
                        program_filter,
                        tty_filter,
                    )? {
                        processes_with_stats.push((process, stat));
                    }
                }
            }
        }

        Ok(processes_with_stats)
    }
}

#[derive(Default, Debug, Clone)]
pub struct ProcessStat {
    pub state: ProcessState,
    pub comm: String,
    pub ppid: u32,
    pub pgrp: u32,
    pub session: u32,
    pub tty_nr_major: u8,
    pub tty_nr_minor: u32,
    pub tpgid: Option<u32>,
    pub utime: u32,
    pub stime: u32,
    pub cutime: u32,
    pub cstime: u32,
    pub priority: i8,
    pub nice: i8,
    pub num_threads: usize,
    pub starttime: u64,
    /// size, VmSize (total program size)
    pub vsize: usize,
    /// resident, VmRSS (resident set size)
    pub rss: usize,
    pub rsslim: usize,
    pub processor: usize,
    pub rt_priority: u8,
    /// RssFile + RssShmem (number of resident shared pages)
    pub shared: usize,
    /// VmRSS - RssFile - RssShmem = RssAnon (resident anonymous memory, process occupied memory)
    pub rss_anon: usize,
}

impl ProcessStat {
    pub fn get_process_stat(pid: u32) -> Result<ProcessStat, ScannerError> {
        let mut stat = ProcessStat::default();

        let stat_path = Path::new("/proc").join(pid.to_string()).join("stat");

        let mut sc = Scanner::scan_path(stat_path)?;

        if sc.drop_next()?.is_none() {
            return Err(ScannerError::IOError(io::Error::new(
                ErrorKind::UnexpectedEof,
                "The format of process.stat (pid) is not correct.",
            )));
        }

        {
            let cont = match sc.next()? {
                Some(comm) => {
                    if comm.starts_with('(') {
                        if comm.ends_with(')') {
                            stat.comm.push_str(&comm[1..(comm.len() - 1)]);
                            false
                        } else {
                            stat.comm.push_str(&comm[1..]);
                            true
                        }
                    } else {
                        return Err(ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The format of process.stat (comm) is not correct.",
                        )));
                    }
                }
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The format of process.stat (comm) is not correct.",
                    )));
                }
            };

            if cont {
                loop {
                    match sc.next()? {
                        Some(comm) => {
                            stat.comm.push(' ');

                            if comm.ends_with(')') {
                                stat.comm.push_str(&comm[..(comm.len() - 1)]);
                                break;
                            } else {
                                stat.comm.push_str(comm.as_str());
                            }
                        }
                        None => {
                            return Err(ScannerError::IOError(io::Error::new(
                                ErrorKind::UnexpectedEof,
                                "The format of process.stat (comm) is not correct.",
                            )));
                        }
                    }
                }
            }
        }

        match sc.next()? {
            Some(state) => {
                stat.state = ProcessState::from_str(state).ok_or_else(|| {
                    ScannerError::IOError(io::Error::new(
                        ErrorKind::InvalidInput,
                        "The format of process.stat (state) is not correct.",
                    ))
                })?;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (state) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(ppid) => {
                stat.ppid = ppid;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (ppid) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(pgrp) => {
                stat.pgrp = pgrp;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (pgrp) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(session) => {
                stat.session = session;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (session) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(tty_nr) => {
                stat.tty_nr_major = (tty_nr >> 8) as u8;
                stat.tty_nr_minor = ((tty_nr >> 20) << 8) | (tty_nr & 0xFF);
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (tty_nr) is not correct.",
                )));
            }
        }

        match sc.next_i32()? {
            Some(tpgid) => {
                if tpgid >= 0 {
                    stat.tpgid = Some(tpgid as u32);
                }
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (tpgid) is not correct.",
                )));
            }
        }

        for item in &["flags", "minflt", "cminflt", "majflt", "cmajflt"] {
            if sc.drop_next()?.is_none() {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    format!("The format of process.stat ({}) is not correct.", item),
                )));
            }
        }

        match sc.next_u32()? {
            Some(utime) => {
                stat.utime = utime;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (utime) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(stime) => {
                stat.stime = stime;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (stime) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(cutime) => {
                stat.cutime = cutime;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (cutime) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(cstime) => {
                stat.cstime = cstime;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (cstime) is not correct.",
                )));
            }
        }

        match sc.next_i8()? {
            Some(priority) => {
                stat.priority = priority;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (priority) is not correct.",
                )));
            }
        }

        match sc.next_i8()? {
            Some(nice) => {
                stat.nice = nice;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (nice) is not correct.",
                )));
            }
        }

        match sc.next_usize()? {
            Some(num_threads) => {
                stat.num_threads = num_threads;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (num_threads) is not correct.",
                )));
            }
        }

        if sc.drop_next()?.is_none() {
            return Err(ScannerError::IOError(io::Error::new(
                ErrorKind::UnexpectedEof,
                "The format of process.stat (itrealvalue) is not correct.",
            )));
        }

        match sc.next_u64()? {
            Some(starttime) => {
                stat.starttime = starttime;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (starttime) is not correct.",
                )));
            }
        }

        match sc.next_usize()? {
            Some(vsize) => {
                stat.vsize = vsize;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (vsize) is not correct.",
                )));
            }
        }

        match sc.next_usize()? {
            Some(rss) => {
                stat.rss = rss;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (rss) is not correct.",
                )));
            }
        }

        match sc.next_usize()? {
            Some(rsslim) => {
                stat.rsslim = rsslim;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (rsslim) is not correct.",
                )));
            }
        }

        for item in &[
            "startcode",
            "endcode",
            "startstack",
            "kstkesp",
            "kstkeip",
            "signal",
            "blocked",
            "sigignore",
            "sigcatch",
            "wchan",
            "nswap",
            "cnswap",
            "exit_signal",
        ] {
            if sc.drop_next()?.is_none() {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    format!("The format of process.stat ({}) is not correct.", item),
                )));
            }
        }

        match sc.next_usize()? {
            Some(processor) => {
                stat.processor = processor;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (processor) is not correct.",
                )));
            }
        }

        match sc.next_u8()? {
            Some(rt_priority) => {
                stat.rt_priority = rt_priority;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (rt_priority) is not correct.",
                )));
            }
        }

        drop(sc);

        let stat_path = Path::new("/proc").join(pid.to_string()).join("statm");

        let mut sc = Scanner::scan_path(stat_path)?;

        for item in &["size", "resident"] {
            if sc.drop_next()?.is_none() {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    format!("The format of process.statm ({}) is not correct.", item),
                )));
            }
        }

        match sc.next_usize()? {
            Some(shared) => {
                stat.shared = shared;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.statm (shared) is not correct.",
                )));
            }
        }

        stat.rss_anon = stat.rss - stat.shared;

        Ok(stat)
    }
}

#[derive(Default, Debug, Clone)]
pub struct ProcessStatSelfTime {
    pub utime: u32,
    pub stime: u32,
}

impl ProcessStatSelfTime {
    pub fn get_process_stat_self_time(pid: u32) -> Result<ProcessStatSelfTime, ScannerError> {
        let mut stat = ProcessStatSelfTime::default();

        let stat_path = Path::new("/proc").join(pid.to_string()).join("stat");

        let mut sc = Scanner::scan_path(stat_path)?;

        if sc.drop_next()?.is_none() {
            return Err(ScannerError::IOError(io::Error::new(
                ErrorKind::UnexpectedEof,
                "The format of process.stat (pid) is not correct.",
            )));
        }

        loop {
            match sc.next()? {
                Some(comm) => {
                    if comm.ends_with(')') {
                        break;
                    }
                }
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The format of process.stat (comm) is not correct.",
                    )));
                }
            }
        }

        for item in &[
            "state", "ppid", "pgrp", "session", "tty_nr", "tpgid", "flags", "minflt", "cminflt",
            "majflt", "cmajflt",
        ] {
            if sc.drop_next()?.is_none() {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    format!("The format of process.stat ({}) is not correct.", item),
                )));
            }
        }

        match sc.next_u32()? {
            Some(utime) => {
                stat.utime = utime;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (utime) is not correct.",
                )));
            }
        }

        match sc.next_u32()? {
            Some(stime) => {
                stat.stime = stime;
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of process.stat (stime) is not correct.",
                )));
            }
        }

        Ok(stat)
    }
}

impl Process {
    #[inline]
    pub fn compute_percentage(
        total_cpu_time: f64,
        pre_process_stat: ProcessStat,
        process_stat_self_time: ProcessStatSelfTime,
    ) -> f64 {
        let d_utime = process_stat_self_time.utime - pre_process_stat.utime;
        let d_stime = process_stat_self_time.stime - pre_process_stat.stime;
        let d_time_f64 = (d_utime + d_stime) as f64;

        if total_cpu_time < 1.0 {
            0.0
        } else if d_time_f64 >= total_cpu_time {
            1.0
        } else {
            d_time_f64 / total_cpu_time
        }
    }

    pub fn get_processes_with_percentage(
        processes_with_stats: Vec<(Process, ProcessStat)>,
        interval: Duration,
    ) -> Result<Vec<(Process, f64)>, ScannerError> {
        let pre_cpus_stat = CPUStat::get_average_cpu_stat()?;
        let mut process_with_percentage = Vec::with_capacity(processes_with_stats.len());

        sleep(interval);

        let cpus_stat = CPUStat::get_average_cpu_stat()?;

        let total_cpu_time_f64 = {
            let (_, _, pre_total) = pre_cpus_stat.compute_time();
            let (_, _, total) = cpus_stat.compute_time();

            (total - pre_total) as f64
        };

        for (process, pre_process_stat) in processes_with_stats {
            if let Ok(process_stat_self_time) =
                ProcessStatSelfTime::get_process_stat_self_time(process.pid)
            {
                let percentage = Process::compute_percentage(
                    total_cpu_time_f64,
                    pre_process_stat,
                    process_stat_self_time,
                );

                process_with_percentage.push((process, percentage));
            }
        }

        Ok(process_with_percentage)
    }
}

#[derive(Default, Debug, Clone)]
pub struct ProcessStatus {
    /// The user who created this process or the UID set via `setuid()` by the root caller.
    pub real_uid: u32,
    /// The group who created this process or the GID set via `setgid()` by the root caller.
    pub real_gid: u32,
    /// The UID set via `setuid()` by the caller.
    pub effective_uid: u32,
    /// The GID set via `setgid()` by the caller.
    pub effective_gid: u32,
    /// The UID set via `setuid()` by the root caller
    pub saved_set_uid: u32,
    /// The GID set via `setgid()` by the root caller
    pub saved_set_gid: u32,
    /// The UID of the running executable file of this process.
    pub fs_uid: u32,
    /// The GID of the running executable file of this process.
    pub fs_gid: u32,
}

impl ProcessStatus {
    pub fn get_process_status(pid: u32) -> Result<ProcessStatus, ScannerError> {
        let mut stat = ProcessStatus::default();

        let stat_path = Path::new("/proc").join(pid.to_string()).join("status");

        let mut sc = Scanner::scan_path(stat_path)?;

        loop {
            let label = sc.next()?;

            match label {
                Some(label) => {
                    if label.starts_with("Uid") {
                        match sc.next_u32()? {
                            Some(value) => {
                                stat.real_uid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `real_uid` is not correct.",
                                )));
                            }
                        }

                        match sc.next_u32()? {
                            Some(value) => {
                                stat.effective_uid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `effective_uid` is not correct.",
                                )));
                            }
                        }

                        match sc.next_u32()? {
                            Some(value) => {
                                stat.saved_set_uid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `saved_set_uid` is not correct.",
                                )));
                            }
                        }

                        match sc.next_u32()? {
                            Some(value) => {
                                stat.fs_uid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `fs_uid` is not correct.",
                                )));
                            }
                        }

                        break;
                    } else if sc.drop_next_line()?.is_none() {
                        return Err(ScannerError::IOError(io::Error::new(
                            ErrorKind::UnexpectedEof,
                            "The format of item `uid` is correct.",
                        )));
                    }
                }
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The item `uid` is not found.",
                    )));
                }
            }
        }

        loop {
            let label = sc.next()?;

            match label {
                Some(label) => {
                    if label.starts_with("Gid") {
                        match sc.next_u32()? {
                            Some(value) => {
                                stat.real_gid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `real_gid` is not correct.",
                                )));
                            }
                        }

                        match sc.next_u32()? {
                            Some(value) => {
                                stat.effective_gid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `effective_gid` is not correct.",
                                )));
                            }
                        }

                        match sc.next_u32()? {
                            Some(value) => {
                                stat.saved_set_gid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `saved_set_gid` is not correct.",
                                )));
                            }
                        }

                        match sc.next_u32()? {
                            Some(value) => {
                                stat.fs_gid = value;
                            }
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "The format of item `fs_gid` is not correct.",
                                )));
                            }
                        }

                        break;
                    } else if sc.drop_next_line()?.is_none() {
                        return Err(ScannerError::IOError(io::Error::new(
                            ErrorKind::UnexpectedEof,
                            "The format of item `gid` is correct.",
                        )));
                    }
                }
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The item `gid` is not found.",
                    )));
                }
            }
        }

        Ok(stat)
    }
}
