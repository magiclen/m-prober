use std::collections::btree_set::BTreeSet;
use std::io::{self, ErrorKind};
use std::thread::sleep;
use std::time::Duration;

use crate::scanner_rust::{Scanner, ScannerError};

const CPUINFO_PATH: &str = "/proc/cpuinfo";

const ITEMS: [&str; 5] = ["model name", "cpu MHz", "physical id", "siblings", "cpu cores"];
const MODEL_NAME_INDEX: usize = 0;
const CPU_MHZ_INDEX: usize = 1;
const PHYSICAL_ID_INDEX: usize = 2;
const SIBLINGS_INDEX: usize = 3;
const CPU_CORES: usize = 4;

const STAT_PATH: &str = "/proc/stat";

#[derive(Debug, Clone)]
pub struct CPU {
    pub physical_id: usize,
    pub model_name: String,
    pub cpus_mhz: Vec<f64>,
    pub siblings: usize,
    pub cpu_cores: usize,
}

impl CPU {
    pub fn get_cpus() -> Result<Vec<CPU>, ScannerError> {
        let mut sc = Scanner::scan_path(CPUINFO_PATH)?;

        let mut cpus = Vec::with_capacity(1);
        let mut physical_ids: BTreeSet<usize> = BTreeSet::new();

        let mut physical_id = 0;
        let mut model_name = String::new();
        let mut cpus_mhz = Vec::with_capacity(1);
        let mut siblings = 0;
        let mut cpu_cores = 0;

        'outer: loop {
            'item: for (i, &item) in ITEMS.iter().enumerate() {
                let item_len = item.len();

                loop {
                    match sc.next_line()? {
                        Some(line) => {
                            if line.starts_with(item) {
                                match line[item_len..].find(':') {
                                    Some(colon_index) => {
                                        let value = line[(item_len + colon_index + 1)..].trim();

                                        match i {
                                            MODEL_NAME_INDEX => {
                                                if model_name.is_empty() {
                                                    model_name.push_str(value);
                                                }
                                            }
                                            CPU_MHZ_INDEX => {
                                                let cpu_mhz: f64 = value.parse().map_err(|_| {
                                                    ScannerError::IOError(io::Error::new(
                                                        ErrorKind::InvalidInput,
                                                        format!(
                                                            "The item `{}` has an incorrect value.",
                                                            item
                                                        ),
                                                    ))
                                                })?;

                                                cpus_mhz.push(cpu_mhz);
                                            }
                                            PHYSICAL_ID_INDEX => {
                                                physical_id = value.parse().map_err(|_| {
                                                    ScannerError::IOError(io::Error::new(
                                                        ErrorKind::InvalidInput,
                                                        format!(
                                                            "The item `{}` has an incorrect value.",
                                                            item
                                                        ),
                                                    ))
                                                })?;

                                                if physical_ids.contains(&physical_id) {
                                                    break 'item;
                                                }
                                            }
                                            SIBLINGS_INDEX => {
                                                siblings = value.parse().map_err(|_| {
                                                    ScannerError::IOError(io::Error::new(
                                                        ErrorKind::InvalidInput,
                                                        format!(
                                                            "The item `{}` has an incorrect value.",
                                                            item
                                                        ),
                                                    ))
                                                })?;
                                            }
                                            CPU_CORES => {
                                                cpu_cores = value.parse().map_err(|_| {
                                                    ScannerError::IOError(io::Error::new(
                                                        ErrorKind::InvalidInput,
                                                        format!(
                                                            "The item `{}` has an incorrect value.",
                                                            item
                                                        ),
                                                    ))
                                                })?;

                                                break 'item;
                                            }
                                            _ => unreachable!(),
                                        }
                                    }
                                    None => {
                                        return Err(ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `{}` has no colon.", item),
                                        )));
                                    }
                                }

                                break;
                            }
                        }
                        None => {
                            if i == MODEL_NAME_INDEX {
                                break 'outer;
                            } else {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    format!("The item `{}` is not found.", item),
                                )));
                            }
                        }
                    }
                }
            }

            if siblings == cpus_mhz.len() {
                let cpu = CPU {
                    physical_id,
                    model_name,
                    cpus_mhz,
                    siblings,
                    cpu_cores,
                };

                cpus.push(cpu);
                physical_ids.insert(physical_id);

                physical_id = 0;
                model_name = String::new();
                cpus_mhz = Vec::with_capacity(1);
                siblings = 0;
                cpu_cores = 0;
            }

            loop {
                let line_length = sc.drop_next_line()?;

                match line_length {
                    Some(line_length) => {
                        if line_length == 0 {
                            break;
                        }
                    }
                    None => {
                        break 'outer;
                    }
                }
            }
        }

        Ok(cpus)
    }
}

#[derive(Debug, Clone)]
pub struct CPUStat {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guest_nice: u64,
}

impl CPUStat {
    pub fn get_average_cpu_stat() -> Result<CPUStat, ScannerError> {
        let mut sc = Scanner::scan_path(STAT_PATH)?;

        let label = sc.next()?;

        match label {
            Some(label) => {
                if label.as_str().eq("cpu") {
                    let user = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let nice = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let system = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let idle = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let iowait = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let irq = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let softirq = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let steal = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let guest = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;
                    let guest_nice = sc.next_u64()?.ok_or_else(|| {
                        ScannerError::IOError(io::Error::new(
                            ErrorKind::InvalidInput,
                            "The item `cpu` has an incorrect value.",
                        ))
                    })?;

                    Ok(CPUStat {
                        user,
                        nice,
                        system,
                        idle,
                        iowait,
                        irq,
                        softirq,
                        steal,
                        guest,
                        guest_nice,
                    })
                } else {
                    Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The item `cpu` is not found.",
                    )))
                }
            }
            None => {
                Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The item `cpu` is not found.",
                )))
            }
        }
    }

    pub fn get_all_cpus_stat(with_average: bool) -> Result<Vec<CPUStat>, ScannerError> {
        let mut sc = Scanner::scan_path(STAT_PATH)?;

        let label = sc.next()?;

        let mut cpus_stat = Vec::with_capacity(1);

        match label {
            Some(label) => {
                if label.as_str().eq("cpu") {
                    if with_average {
                        let user = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let nice = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let system = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let idle = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let iowait = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let irq = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let softirq = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let steal = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let guest = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;
                        let guest_nice = sc.next_u64()?.ok_or_else(|| {
                            ScannerError::IOError(io::Error::new(
                                ErrorKind::InvalidInput,
                                "The item `cpu` has an incorrect value.",
                            ))
                        })?;

                        let cpu_stat = CPUStat {
                            user,
                            nice,
                            system,
                            idle,
                            iowait,
                            irq,
                            softirq,
                            steal,
                            guest,
                            guest_nice,
                        };

                        cpus_stat.push(cpu_stat);
                    } else if sc.drop_next_line()?.is_none() {
                        return Err(ScannerError::IOError(io::Error::new(
                            ErrorKind::UnexpectedEof,
                            "The format of item `cpu` is correct.",
                        )));
                    }

                    let mut i = 0;

                    loop {
                        let label = sc.next()?;

                        match label {
                            Some(label) => {
                                if label.starts_with("cpu") {
                                    let user = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let nice = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let system = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let idle = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let iowait = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let irq = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let softirq = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let steal = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let guest = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;
                                    let guest_nice = sc.next_u64()?.ok_or_else(|| {
                                        ScannerError::IOError(io::Error::new(
                                            ErrorKind::InvalidInput,
                                            format!("The item `cpu{}` has an incorrect value.", i),
                                        ))
                                    })?;

                                    let cpu_stat = CPUStat {
                                        user,
                                        nice,
                                        system,
                                        idle,
                                        iowait,
                                        irq,
                                        softirq,
                                        steal,
                                        guest,
                                        guest_nice,
                                    };

                                    cpus_stat.push(cpu_stat);

                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                } else {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The item `cpu` is not found.",
                    )));
                }
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The item `cpu` is not found.",
                )));
            }
        }

        if cpus_stat.is_empty() {
            return Err(ScannerError::IOError(io::Error::new(
                ErrorKind::InvalidInput,
                "Cannot get information of all CPUs.",
            )));
        }

        Ok(cpus_stat)
    }
}

impl CPUStat {
    #[inline]
    pub fn compute_time(&self) -> (u64, u64, u64) {
        let idle = self.idle + self.iowait;

        let non_idle = self.user + self.nice + self.system + self.irq + self.softirq + self.steal;

        let total = idle + non_idle;

        (non_idle, idle, total)
    }

    #[inline]
    pub fn compute_percentage(pre_cpu_stat: CPUStat, cpu_stat: CPUStat) -> f64 {
        let (non_idle, _, total) = cpu_stat.compute_time();
        let (pre_non_idle, _, pre_total) = pre_cpu_stat.compute_time();

        let d_total = total - pre_total;
        let d_non_idle = non_idle - pre_non_idle;

        d_non_idle as f64 / d_total as f64
    }

    #[inline]
    pub fn get_average_percentage(interval: Duration) -> Result<f64, ScannerError> {
        let pre_cpu_stat = CPUStat::get_average_cpu_stat()?;

        sleep(interval);

        let cpu_stat = CPUStat::get_average_cpu_stat()?;

        Ok(CPUStat::compute_percentage(pre_cpu_stat, cpu_stat))
    }

    pub fn get_all_percentage(
        with_average: bool,
        interval: Duration,
    ) -> Result<Vec<f64>, ScannerError> {
        let pre_cpus_stat = CPUStat::get_all_cpus_stat(with_average)?;

        sleep(interval);

        let mut cpus_stat = CPUStat::get_all_cpus_stat(with_average)?;

        let cpus_stat_len = cpus_stat.len();

        let mut result = Vec::with_capacity(cpus_stat_len);

        unsafe {
            result.set_len(cpus_stat_len);
        }

        for (i, pre_cpu_stat) in pre_cpus_stat.into_iter().enumerate().rev() {
            let cpu_stat = cpus_stat.pop().unwrap();

            result[i] = CPUStat::compute_percentage(pre_cpu_stat, cpu_stat);
        }

        Ok(result)
    }
}
