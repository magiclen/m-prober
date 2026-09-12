use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    thread,
    time::Duration,
};

use mprober_lib::{Error, cpu::CPUStat};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CpuThreadSnapshot {
    pub id:            usize,
    pub physical_id:   Option<usize>,
    pub usage:         Option<f64>,
    pub frequency_mhz: Option<f64>,
}

pub struct CpuSample {
    pub cpus_stat: Vec<f64>,
    pub threads:   Vec<CpuThreadSnapshot>,
}

struct Counts {
    average: CPUStat,
    threads: BTreeMap<usize, CPUStat>,
}

#[derive(Default)]
struct ThreadInfo {
    physical_id:   Option<usize>,
    frequency_mhz: Option<f64>,
}

fn read_counts(reader: impl BufRead) -> io::Result<Counts> {
    let mut average = None;
    let mut threads = BTreeMap::new();

    for line in reader.lines() {
        let line = line?;
        let mut fields = line.split_ascii_whitespace();
        let Some(label) = fields.next() else { continue };
        let Some(suffix) = label.strip_prefix("cpu") else { break };
        let mut values = [0; 10];
        for value in &mut values {
            *value = fields
                .next()
                .ok_or(io::ErrorKind::UnexpectedEof)?
                .parse::<u64>()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        }
        let stat = CPUStat {
            user:       values[0],
            nice:       values[1],
            system:     values[2],
            idle:       values[3],
            iowait:     values[4],
            irq:        values[5],
            softirq:    values[6],
            steal:      values[7],
            guest:      values[8],
            guest_nice: values[9],
        };
        if suffix.is_empty() {
            average = Some(stat);
        } else {
            let id = suffix
                .parse::<usize>()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            threads.insert(id, stat);
        }
    }

    Ok(Counts {
        average: average.ok_or(io::ErrorKind::InvalidData)?,
        threads,
    })
}

fn parse_info(data: &str) -> BTreeMap<usize, ThreadInfo> {
    let mut result = BTreeMap::<usize, ThreadInfo>::new();
    let mut id = None;
    for line in data.lines() {
        if line.trim().is_empty() {
            id = None;
            continue;
        }
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key.trim() {
            "processor" => {
                id = value.parse::<usize>().ok();
                if let Some(id) = id {
                    result.entry(id).or_default();
                }
            },
            "physical id" => {
                if let Some(id) = id {
                    result.entry(id).or_default().physical_id = value.parse().ok();
                }
            },
            "cpu MHz" => {
                if let Some(id) = id {
                    result.entry(id).or_default().frequency_mhz =
                        value.parse::<f64>().ok().filter(|mhz| mhz.is_finite() && *mhz > 0.0);
                }
            },
            _ => (),
        }
    }
    result
}

fn combine(before: Counts, after: Counts, info: &BTreeMap<usize, ThreadInfo>) -> CpuSample {
    let mut cpus_stat = vec![before.average.compute_cpu_utilization_in_percentage(&after.average)];
    let threads = after
        .threads
        .into_iter()
        .map(|(id, stat)| {
            // Match the same kernel CPU number even when CPUs go offline between samples.
            let usage = before
                .threads
                .get(&id)
                .map(|previous| previous.compute_cpu_utilization_in_percentage(&stat));
            let info = info.get(&id);
            // The legacy array cannot represent a missing first reading; the new field can.
            cpus_stat.push(usage.unwrap_or(0.0));
            CpuThreadSnapshot {
                id,
                physical_id: info.and_then(|info| info.physical_id),
                usage,
                frequency_mhz: info.and_then(|info| info.frequency_mhz),
            }
        })
        .collect();
    CpuSample {
        cpus_stat,
        threads,
    }
}

pub fn sample(interval: Duration) -> Result<CpuSample, Error> {
    let before = read_counts(BufReader::new(File::open("/proc/stat")?))?;
    thread::sleep(interval);
    let after = read_counts(BufReader::new(File::open("/proc/stat")?))?;
    let mut info = parse_info(&fs::read_to_string("/proc/cpuinfo").unwrap_or_default());
    for id in after.threads.keys() {
        let entry = info.entry(*id).or_default();
        // Read only the missing attributes, not every file in the topology and cpufreq folders.
        if entry.physical_id.is_none() {
            entry.physical_id = fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpu{id}/topology/physical_package_id"
            ))
            .ok()
            .and_then(|value| value.trim().parse().ok());
        }
        if entry.frequency_mhz.is_none() {
            entry.frequency_mhz = fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpu{id}/cpufreq/scaling_cur_freq"
            ))
            .ok()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|khz| khz.is_finite() && *khz > 0.0)
            .map(|khz| khz / 1000.0);
        }
    }
    Ok(combine(before, after, &info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_cpu() {
        let before =
            read_counts("cpu 0 0 0 0 0 0 0 0 0 0\ncpu0 0 0 0 0 0 0 0 0 0 0\n".as_bytes()).unwrap();
        let after = read_counts(
            "cpu 25 0 0 75 0 0 0 0 0 0\ncpu0 25 0 0 75 0 0 0 0 0 0\nintr 0\n".as_bytes(),
        )
        .unwrap();
        let info = parse_info("processor : 0\nphysical id : 0\ncpu MHz : 2830.00\n");
        let sample = combine(before, after, &info);
        assert_eq!(vec![0.25, 0.25], sample.cpus_stat);
        assert_eq!(1, sample.threads.len());
        assert_eq!(Some(0), sample.threads[0].physical_id);
        assert_eq!(Some(2830.0), sample.threads[0].frequency_mhz);
    }

    #[test]
    fn test_two_packages_with_non_contiguous_cpu_numbers() {
        let before = read_counts(
            "cpu 0 0 0 0 0 0 0 0 0 0\ncpu1 0 0 0 0 0 0 0 0 0 0\ncpu4 0 0 0 0 0 0 0 0 0 0\ncpu9 0 \
             0 0 0 0 0 0 0 0 0\n"
                .as_bytes(),
        )
        .unwrap();
        let after = read_counts(
            "cpu 100 0 0 100 0 0 0 0 0 0\ncpu4 75 0 0 25 0 0 0 0 0 0\ncpu1 25 0 0 75 0 0 0 0 0 0\n"
                .as_bytes(),
        )
        .unwrap();
        let info = parse_info(
            "processor : 4\nphysical id : 0\ncpu MHz : 3200\n\nprocessor : 1\nphysical id : 1\n",
        );
        let sample = combine(before, after, &info);
        assert_eq!(vec![0.5, 0.25, 0.75], sample.cpus_stat);
        assert_eq!(2, sample.threads.len());
        assert_eq!(1, sample.threads[0].id);
        assert_eq!(Some(1), sample.threads[0].physical_id);
        assert_eq!(None, sample.threads[0].frequency_mhz);
        assert_eq!(4, sample.threads[1].id);
        assert_eq!(Some(0), sample.threads[1].physical_id);
        assert_eq!(Some(3200.0), sample.threads[1].frequency_mhz);
    }

    #[test]
    fn test_new_cpu_keeps_an_unknown_usage() {
        let before = read_counts("cpu 0 0 0 0 0 0 0 0 0 0\n".as_bytes()).unwrap();
        let after =
            read_counts("cpu 1 0 0 1 0 0 0 0 0 0\ncpu8 1 0 0 1 0 0 0 0 0 0\n".as_bytes()).unwrap();
        let sample = combine(before, after, &BTreeMap::new());
        assert_eq!(8, sample.threads[0].id);
        assert_eq!(None, sample.threads[0].usage);
        assert_eq!(None, sample.threads[0].physical_id);
    }
}
