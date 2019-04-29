mod mounts;

use std::hash::{Hash, Hasher};
use std::collections::HashSet;
use std::io::{self, ErrorKind};
use std::time::Duration;
use std::thread::sleep;
use std::mem;
use std::ffi::CString;

use crate::scanner_rust::{Scanner, ScannerError};

const DISKSTATS_PATH: &'static str = "/proc/diskstats";

const SECTOR_SIZE: u64 = 512;

#[derive(Debug, Clone, Eq)]
pub struct Disk {
    pub device: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub size: u64,
    pub used: u64,
    pub points: Vec<String>,
}

impl Hash for Disk {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.device.hash(state)
    }
}

impl PartialEq for Disk {
    #[inline]
    fn eq(&self, other: &Disk) -> bool {
        self.device.eq(&other.device)
    }

    #[inline]
    fn ne(&self, other: &Disk) -> bool {
        self.device.ne(&other.device)
    }
}

impl Disk {
    pub fn get_disks() -> Result<Vec<Disk>, ScannerError> {
        let mut mounts = mounts::get_mounts()?;

        let mut sc = Scanner::scan_path(DISKSTATS_PATH)?;

        let mut disks = Vec::with_capacity(1);

        loop {
            if sc.next()?.is_none() {
                break;
            }

            if sc.next()?.is_none() {
                return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The format of disk stats is not correct.".to_string())));
            }

            match sc.next()? {
                Some(device) => {
                    if let Some(points) = mounts.remove(&device) {
                        for _ in 0..2 {
                            if sc.next_u64()?.is_none() {
                                return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of device `{}` is not correct.", device))));
                            }
                        }

                        let read_bytes = match sc.next_u64()? {
                            Some(sectors) => {
                                sectors * SECTOR_SIZE
                            }
                            None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of device `{}` is not correct.", device))))
                        };

                        for _ in 0..3 {
                            if sc.next_u64()?.is_none() {
                                return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of device `{}` is not correct.", device))));
                            }
                        }

                        let write_bytes = match sc.next_u64()? {
                            Some(sectors) => {
                                sectors * SECTOR_SIZE
                            }
                            None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of device `{}` is not correct.", device))))
                        };

                        for _ in 0..2 {
                            if sc.next_u64()?.is_none() {
                                return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of device `{}` is not correct.", device))));
                            }
                        }

                        let time_spent = match sc.next_u64()? {
                            Some(milliseconds) => milliseconds,
                            None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of device `{}` is not correct.", device))))
                        };

                        if time_spent > 0 {
                            let (size, used) = {
                                let path = CString::new(points[0].as_bytes()).unwrap();

                                let mut stats: libc::statvfs = unsafe { mem::zeroed() };

                                let rtn = unsafe { libc::statvfs(path.as_ptr(), &mut stats as *mut _) };

                                if rtn != 0 {
                                    return Err(ScannerError::IOError(io::Error::new(ErrorKind::Other, format!("Cannot get the stats of the path `{}`.", points[0]))));
                                }

                                (stats.f_bsize * stats.f_blocks, stats.f_bsize * (stats.f_blocks - stats.f_bavail))
                            };

                            let disk = Disk {
                                device,
                                read_bytes,
                                write_bytes,
                                size,
                                used,
                                points,
                            };

                            disks.push(disk);
                        }
                    }

                    if sc.next_line()?.is_none() {
                        return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The format of disk stats is not correct.".to_string())));
                    }
                }
                None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The format of disk stats is not correct.".to_string())))
            }
        }

        Ok(disks)
    }
}

#[derive(Debug, Clone)]
pub struct Speed {
    pub read: f64,
    pub write: f64,
}

#[derive(Debug, Clone)]
pub struct DiskWithSpeed {
    pub disk: Disk,
    pub speed: Speed,
}

impl DiskWithSpeed {
    pub fn get_disks_with_speed(interval: Duration) -> Result<Vec<DiskWithSpeed>, ScannerError> {
        let mut pre_disks = Disk::get_disks()?;

        let pre_disk_len = pre_disks.len();

        let mut pre_disks_hashset = HashSet::with_capacity(pre_disk_len);

        loop {
            match pre_disks.pop() {
                Some(network) => {
                    pre_disks_hashset.insert(network);
                }
                None => break
            }
        }

        let seconds = interval.as_secs_f64();

        sleep(interval);

        let disks = Disk::get_disks()?;

        let mut result = Vec::with_capacity(disks.len().min(pre_disk_len));

        for disk in disks {
            if let Some(pre_disk) = pre_disks_hashset.get(&disk) {
                let d_read = disk.read_bytes - pre_disk.read_bytes;
                let d_write = disk.write_bytes - pre_disk.write_bytes;

                let read = d_read as f64 / seconds;
                let write = d_write as f64 / seconds;

                let speed = Speed {
                    read,
                    write,
                };

                let disk_with_speed = DiskWithSpeed {
                    disk,
                    speed,
                };

                result.push(disk_with_speed);
            }
        }

        Ok(result)
    }
}