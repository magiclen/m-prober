use std::io::{self, ErrorKind};

use crate::scanner_rust::{Scanner, ScannerError};

const MEMINFO_PATH: &str = "/proc/meminfo";
const ITEMS: [&str; 11] = [
    "MemTotal",
    "MemFree",
    "MemAvailable",
    "Buffers",
    "Cached",
    "SwapCached",
    "SwapTotal",
    "SwapFree",
    "Shmem",
    "Slab",
    "SUnreclaim",
];

#[derive(Debug, Clone)]
pub struct Mem {
    pub total: usize,
    /// total - free - buffers - cached - total_cached; total_cached = cached + slab - s_unreclaim
    pub used: usize,
    pub free: usize,
    pub shared: usize,
    pub buffers: usize,
    pub cache: usize,
    pub available: usize,
}

#[derive(Debug, Clone)]
pub struct Swap {
    pub total: usize,
    /// swap_total - swap_free - swap_cached
    pub used: usize,
    pub free: usize,
    pub cache: usize,
}

#[derive(Debug, Clone)]
pub struct Free {
    pub mem: Mem,
    pub swap: Swap,
}

impl Free {
    pub fn get_free() -> Result<Free, ScannerError> {
        let mut sc = Scanner::scan_path(MEMINFO_PATH)?;

        let mut item_values = [0usize; ITEMS.len()];

        for (i, &item) in ITEMS.iter().enumerate() {
            loop {
                match sc.next()? {
                    Some(label) => {
                        if label.starts_with(item) {
                            match sc.next_usize()? {
                                Some(value) => {
                                    item_values[i] = value * 1024;
                                }
                                None => {
                                    return Err(ScannerError::IOError(io::Error::new(
                                        ErrorKind::UnexpectedEof,
                                        format!("The format of item `{}` is not correct.", item),
                                    )));
                                }
                            }

                            break;
                        } else if sc.drop_next_line()?.is_none() {
                            return Err(ScannerError::IOError(io::Error::new(
                                ErrorKind::UnexpectedEof,
                                format!("The format of label `{}` is not correct.", label),
                            )));
                        }
                    }
                    None => {
                        return Err(ScannerError::IOError(io::Error::new(
                            ErrorKind::UnexpectedEof,
                            format!("The item `{}` is not found.", item),
                        )));
                    }
                }
            }
        }

        let total = item_values[0];
        let free = item_values[1];
        let available = item_values[2];
        let buffers = item_values[3];
        let cached = item_values[4];
        let swap_cached = item_values[5];
        let swap_total = item_values[6];
        let swap_free = item_values[7];
        let shmem = item_values[8];
        let slab = item_values[9];
        let s_unreclaim = item_values[10];

        let total_cached = cached + slab - s_unreclaim;

        let mem = Mem {
            total,
            used: total - free - buffers - total_cached,
            free,
            shared: shmem,
            buffers,
            cache: total_cached,
            available,
        };

        let swap = Swap {
            total: swap_total,
            used: swap_total - swap_free - swap_cached,
            free: swap_free,
            cache: swap_cached,
        };

        Ok(Free {
            mem,
            swap,
        })
    }
}
