use std::path::PathBuf;

use mprober_lib::{Error, cgroup, network, pressure, volume};
use serde::Serialize;

/// An interface together with the speed measured over the detect interval.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkWithSpeed {
    #[serde(flatten)]
    pub network: network::Network,
    pub speed:   network::NetworkSpeed,
}

impl NetworkWithSpeed {
    pub fn collect(networks: Vec<(network::Network, network::NetworkSpeed)>) -> Vec<Self> {
        networks
            .into_iter()
            .map(|(network, speed)| NetworkWithSpeed {
                network,
                speed,
            })
            .collect()
    }
}

/// A volume together with the I/O speed measured over the detect interval.
#[derive(Debug, Clone, Serialize)]
pub struct VolumeWithSpeed {
    #[serde(flatten)]
    pub volume: volume::Volume,
    pub speed:  volume::VolumeSpeed,
}

impl VolumeWithSpeed {
    pub fn collect(volumes: Vec<(volume::Volume, volume::VolumeSpeed)>) -> Vec<Self> {
        volumes
            .into_iter()
            .map(|(volume, speed)| VolumeWithSpeed {
                volume,
                speed,
            })
            .collect()
    }
}

/// The PSI of every resource the kernel reports.
#[derive(Debug, Clone, Serialize)]
pub struct SystemPressure {
    pub cpu:    pressure::Pressure,
    pub memory: pressure::Pressure,
    pub io:     pressure::Pressure,
}

impl SystemPressure {
    /// `None` means the kernel was built without `CONFIG_PSI` or it was disabled by the `psi=0` boot parameter.
    pub fn probe() -> Result<Option<Self>, Error> {
        let cpu = match pressure::get_cpu_pressure() {
            Ok(cpu) => cpu,
            Err(error) if error.is_not_supported() => return Ok(None),
            Err(error) => return Err(error),
        };

        Ok(Some(SystemPressure {
            cpu,
            memory: pressure::get_memory_pressure()?,
            io: pressure::get_io_pressure()?,
        }))
    }
}

/// The limits and usage of the cgroup this process runs in, which is what a container or a cloud instance is actually capped by.
#[derive(Debug, Clone, Serialize)]
pub struct CgroupSummary {
    pub path:   PathBuf,
    pub cpu:    Option<cgroup::CgroupCPU>,
    pub memory: Option<cgroup::CgroupMemory>,
    pub pids:   Option<cgroup::CgroupPids>,
}

impl CgroupSummary {
    /// `None` means this process does not run under cgroup v2.
    pub fn probe() -> Result<Option<Self>, Error> {
        let Some(path) = cgroup_path() else {
            return Ok(None);
        };

        Ok(Some(CgroupSummary {
            cpu: optional(cgroup::get_cgroup_cpu(&path))?,
            memory: optional(cgroup::get_cgroup_memory(&path))?,
            pids: optional(cgroup::get_cgroup_pids(&path))?,
            path,
        }))
    }
}

/// Everything the cgroup exposes, for the dedicated endpoint and the CLI.
#[derive(Debug, Clone, Serialize)]
pub struct CgroupReport {
    pub path:          PathBuf,
    pub cpu:           Option<cgroup::CgroupCPU>,
    pub cpuset:        Option<cgroup::CgroupCpuset>,
    pub memory:        Option<cgroup::CgroupMemory>,
    pub memory_stat:   Option<cgroup::CgroupMemoryStat>,
    pub memory_events: Option<cgroup::CgroupMemoryEvents>,
    pub pids:          Option<cgroup::CgroupPids>,
    pub io:            Option<Vec<cgroup::CgroupIO>>,
}

impl CgroupReport {
    /// `None` means this process does not run under cgroup v2.
    pub fn probe() -> Result<Option<Self>, Error> {
        let Some(path) = cgroup_path() else {
            return Ok(None);
        };

        Ok(Some(CgroupReport {
            cpu: optional(cgroup::get_cgroup_cpu(&path))?,
            cpuset: optional(cgroup::get_cgroup_cpuset(&path))?,
            memory: optional(cgroup::get_cgroup_memory(&path))?,
            memory_stat: optional(cgroup::get_cgroup_memory_stat(&path))?,
            memory_events: optional(cgroup::get_cgroup_memory_events(&path))?,
            pids: optional(cgroup::get_cgroup_pids(&path))?,
            io: optional(cgroup::get_cgroup_io(&path))?,
            path,
        }))
    }
}

#[inline]
fn cgroup_path() -> Option<PathBuf> {
    cgroup::get_cgroup_path().ok()
}

/// A controller which is not enabled for the cgroup has no files, which is not an error here.
#[inline]
fn optional<T>(result: Result<T, Error>) -> Result<Option<T>, Error> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.is_not_supported() => Ok(None),
        Err(error) => Err(error),
    }
}
