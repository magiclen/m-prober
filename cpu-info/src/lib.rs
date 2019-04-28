extern crate scanner_rust;

use std::io::{self, ErrorKind};
use std::collections::btree_set::BTreeSet;
use std::time::Duration;
use std::thread::sleep;

use scanner_rust::{Scanner, ScannerError};

const CPUINFO_PATH: &'static str = "/proc/cpuinfo";
const ITEMS: [&'static str; 5] = ["model name", "cpu MHz", "physical id", "siblings", "cpu cores"];
const PHYSICAL_ID_INDEX: usize = 2;
const CPU_MHZ_INDEX: usize = 1;

const STAT_PATH: &'static str = "/proc/stat";

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

        let items_len_dec_dec = ITEMS.len() - 2;

        let mut cpus_mhz = Vec::with_capacity(1);

        'outer: loop {
            let mut item_values: Vec<String> = Vec::with_capacity(items_len_dec_dec);

            let mut physical_id = 0;

            'cpu: for (i, &item) in ITEMS.iter().enumerate() {
                let item_len = item.len();

                loop {
                    let line = sc.next_line()?;

                    match line {
                        Some(line) => {
                            if line.as_str().starts_with(item) {
                                match line[item_len..].find(":") {
                                    Some(colon_index) => {
                                        let value = line[(item_len + colon_index + 1)..].trim().to_string();

                                        match i {
                                            CPU_MHZ_INDEX => {
                                                let cpu_mhz: f64 = value.parse().map_err(|_| ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `{}` has incorrect value.", item))))?;

                                                cpus_mhz.push(cpu_mhz);
                                            }
                                            PHYSICAL_ID_INDEX => {
                                                physical_id = value.parse().map_err(|_| ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `{}` has incorrect value.", item))))?;

                                                if physical_ids.contains(&physical_id) {
                                                    break 'cpu;
                                                }
                                            }
                                            _ => {
                                                item_values.push(value);
                                            }
                                        }
                                    }
                                    None => {
                                        return Err(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `{}` has no colon.", item))));
                                    }
                                }

                                break;
                            }
                        }
                        None => {
                            if item_values.len() > 0 {
                                return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The item `{}` is not found.", item))));
                            } else {
                                break 'outer;
                            }
                        }
                    }
                }
            }

            if item_values.len() == items_len_dec_dec {
                let cpu_cores: usize = item_values.pop().unwrap().parse().map_err(|_| ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu cores` has incorrect value.".to_string())))?;
                let siblings: usize = item_values.pop().unwrap().parse().map_err(|_| ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `siblings` has incorrect value.".to_string())))?;

                if siblings == cpus_mhz.len() {
                    let model_name = item_values.pop().unwrap();

                    let cpu = CPU {
                        physical_id,
                        model_name,
                        cpus_mhz,
                        siblings,
                        cpu_cores,
                    };

                    cpus.push(cpu);
                    physical_ids.insert(physical_id);

                    cpus_mhz = Vec::with_capacity(1);
                }
            }

            loop {
                let line = sc.next_line()?;

                match line {
                    Some(line) => {
                        if line.is_empty() {
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
                    let user = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let nice = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let system = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let idle = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let iowait = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let irq = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let softirq = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let steal = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let guest = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;
                    let guest_nice = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The item `cpu` has incorrect value.".to_string())))?;

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
                    Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The item `cpu` is not found.".to_string())))
                }
            }
            None => {
                Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The item `cpu` is not found.".to_string())))
            }
        }
    }

    pub fn get_all_cpus_stat() -> Result<Vec<CPUStat>, ScannerError> {
        let mut sc = Scanner::scan_path(STAT_PATH)?;

        let label = sc.next()?;

        let mut cpus_stat = Vec::with_capacity(1);

        match label {
            Some(label) => {
                if label.as_str().eq("cpu") {
                    sc.next_line()?.unwrap();

                    let mut i = 0;

                    loop {
                        let label = sc.next()?;

                        match label {
                            Some(label) => {
                                if label.starts_with("cpu") {
                                    let user = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let nice = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let system = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let idle = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let iowait = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let irq = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let softirq = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let steal = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let guest = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;
                                    let guest_nice = sc.next_u64()?.ok_or(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `cpu{}` has incorrect value.", i))))?;

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
                    return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The item `cpu` is not found.".to_string())));
                }
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The item `cpu` is not found.".to_string())));
            }
        }

        if cpus_stat.is_empty() {
            return Err(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "Cannot get information of all CPUs.".to_string())));
        }

        Ok(cpus_stat)
    }
}

impl CPUStat {
    fn compute_percentage(pre_cpu_stat: CPUStat, cpu_stat: CPUStat) -> f64 {
        let pre_idle = pre_cpu_stat.idle + pre_cpu_stat.iowait;
        let idle = cpu_stat.idle + cpu_stat.iowait;

        let pre_non_idle = pre_cpu_stat.user + pre_cpu_stat.nice + pre_cpu_stat.system + pre_cpu_stat.irq + pre_cpu_stat.softirq + pre_cpu_stat.steal;
        let non_idle = cpu_stat.user + cpu_stat.nice + cpu_stat.system + cpu_stat.irq + cpu_stat.softirq + cpu_stat.steal;

        let pre_total = pre_idle + pre_non_idle;
        let total = idle + non_idle;

        let d_total = total - pre_total;
        let d_idle = idle - pre_idle;

        (d_total - d_idle) as f64 / d_total as f64
    }

    pub fn get_average_percentage(interval: Duration) -> Result<f64, ScannerError> {
        let pre_cpu_stat = CPUStat::get_average_cpu_stat()?;

        sleep(interval);

        let cpu_stat = CPUStat::get_average_cpu_stat()?;

        Ok(CPUStat::compute_percentage(pre_cpu_stat, cpu_stat))
    }

    pub fn get_all_percentage(interval: Duration) -> Result<Vec<f64>, ScannerError> {
        let pre_cpus_stat = CPUStat::get_all_cpus_stat()?;

        sleep(interval);

        let mut cpus_stat = CPUStat::get_all_cpus_stat()?;

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

#[test]
fn test() {
    println!("{:?}", CPU::get_cpus());
}