mod mounts;

use std::collections::HashSet;
use std::ffi::CString;
use std::hash::{Hash, Hasher};
use std::io::{self, ErrorKind};
use std::mem;
use std::thread::sleep;
use std::time::Duration;

use crate::scanner_rust::{Scanner, ScannerError};

const DISKSTATS_PATH: &str = "/proc/diskstats";

const SECTOR_SIZE: u64 = 512;

#[derive(Debug, Clone, Eq)]
pub struct Volume {
    pub device: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub size: u64,
    pub used: u64,
    pub points: Vec<String>,
}

impl Hash for Volume {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.device.hash(state)
    }
}

impl PartialEq for Volume {
    #[inline]
    fn eq(&self, other: &Volume) -> bool {
        self.device.eq(&other.device)
    }
}

impl Volume {
    pub fn get_volumes() -> Result<Vec<Volume>, ScannerError> {
        let mut mounts = mounts::get_mounts()?;

        let mut sc = Scanner::scan_path(DISKSTATS_PATH)?;

        let mut volumes = Vec::with_capacity(1);

        loop {
            if sc.drop_next()?.is_none() {
                break;
            }

            if sc.drop_next()?.is_none() {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The format of volume stats is not correct.",
                )));
            }

            match sc.next()? {
                Some(device) => {
                    if let Some(points) = mounts.remove(&device) {
                        for _ in 0..2 {
                            if sc.drop_next()?.is_none() {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    format!("The format of device `{}` is not correct.", device),
                                )));
                            }
                        }

                        let read_bytes = match sc.next_u64()? {
                            Some(sectors) => sectors * SECTOR_SIZE,
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    format!("The format of device `{}` is not correct.", device),
                                )))
                            }
                        };

                        for _ in 0..3 {
                            if sc.drop_next()?.is_none() {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    format!("The format of device `{}` is not correct.", device),
                                )));
                            }
                        }

                        let write_bytes = match sc.next_u64()? {
                            Some(sectors) => sectors * SECTOR_SIZE,
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    format!("The format of device `{}` is not correct.", device),
                                )))
                            }
                        };

                        for _ in 0..2 {
                            if sc.drop_next()?.is_none() {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    format!("The format of device `{}` is not correct.", device),
                                )));
                            }
                        }

                        let time_spent = match sc.next_u64()? {
                            Some(milliseconds) => milliseconds,
                            None => {
                                return Err(ScannerError::IOError(io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    format!("The format of device `{}` is not correct.", device),
                                )))
                            }
                        };

                        if time_spent > 0 {
                            let (size, used) = {
                                let path = CString::new(points[0].as_bytes()).unwrap();

                                let mut stats: libc::statvfs = unsafe { mem::zeroed() };

                                let rtn =
                                    unsafe { libc::statvfs(path.as_ptr(), &mut stats as *mut _) };

                                if rtn != 0 {
                                    return Err(ScannerError::IOError(io::Error::new(
                                        ErrorKind::Other,
                                        format!(
                                            "Cannot get the stats of the path `{}`.",
                                            points[0]
                                        ),
                                    )));
                                }

                                (
                                    stats.f_bsize as u64 * stats.f_blocks as u64,
                                    stats.f_bsize as u64 * (stats.f_blocks - stats.f_bavail) as u64,
                                )
                            };

                            let volume = Volume {
                                device,
                                read_bytes,
                                write_bytes,
                                size,
                                used,
                                points,
                            };

                            volumes.push(volume);
                        }
                    }

                    if sc.drop_next_line()?.is_none() {
                        return Err(ScannerError::IOError(io::Error::new(
                            ErrorKind::UnexpectedEof,
                            "The format of volume stats is not correct.",
                        )));
                    }
                }
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The format of volume stats is not correct.",
                    )))
                }
            }
        }

        Ok(volumes)
    }
}

#[derive(Debug, Clone)]
pub struct Speed {
    pub read: f64,
    pub write: f64,
}

#[derive(Debug, Clone)]
pub struct VolumeWithSpeed {
    pub volume: Volume,
    pub speed: Speed,
}

impl VolumeWithSpeed {
    pub fn get_volumes_with_speed(
        interval: Duration,
    ) -> Result<Vec<VolumeWithSpeed>, ScannerError> {
        let mut pre_volumes = Volume::get_volumes()?;

        let pre_volume_len = pre_volumes.len();

        let mut pre_volumes_hashset = HashSet::with_capacity(pre_volume_len);

        while let Some(network) = pre_volumes.pop() {
            pre_volumes_hashset.insert(network);
        }

        let seconds = interval.as_secs_f64();

        sleep(interval);

        let volumes = Volume::get_volumes()?;

        let mut result = Vec::with_capacity(volumes.len().min(pre_volume_len));

        for volume in volumes {
            if let Some(pre_volume) = pre_volumes_hashset.get(&volume) {
                let d_read = volume.read_bytes - pre_volume.read_bytes;
                let d_write = volume.write_bytes - pre_volume.write_bytes;

                let read = d_read as f64 / seconds;
                let write = d_write as f64 / seconds;

                let speed = Speed {
                    read,
                    write,
                };

                let volume_with_speed = VolumeWithSpeed {
                    volume,
                    speed,
                };

                result.push(volume_with_speed);
            }
        }

        Ok(result)
    }
}
