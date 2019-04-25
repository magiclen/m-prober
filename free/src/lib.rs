extern crate scanner_rust;

use std::io::{self, ErrorKind};

use scanner_rust::{Scanner, ScannerError};

const MEMINFO_PATH: &'static str = "/proc/meminfo";
const ITEMS: [&'static str; 11] = ["MemTotal", "MemFree", "MemAvailable", "Buffers", "Cached", "SwapCached", "SwapTotal", "SwapFree", "Shmem", "Slab", "SUnreclaim"];

#[derive(Debug, Clone)]
pub struct Mem {
    total: usize,
    used: usize,
    free: usize,
    shared: usize,
    buffers: usize,
    cache: usize,
    available: usize,
}

#[derive(Debug, Clone)]
pub struct Swap {
    total: usize,
    used: usize,
    free: usize,
}

#[derive(Debug, Clone)]
pub struct Free {
    mem: Mem,
    swap: Swap,
}

impl Free {
    pub fn get_free() -> Result<Free, ScannerError> {
        let mut sc = Scanner::scan_path(MEMINFO_PATH)?;

        let mut item_values = [0usize; 11];

        for (i, &item) in ITEMS.iter().enumerate() {
            loop {
                let label = sc.next()?;

                match label {
                    Some(label) => {
                        if label.as_str().starts_with(item) {
                            let value = sc.next_usize()?;

                            match value {
                                Some(value) => {
                                    item_values[i] = value * 1024;
                                }
                                None => {
                                    return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of item `{}` is not correct.", item))));
                                }
                            }

                            break;
                        } else {
                            sc.next_line()?;
                        }
                    }
                    None => {
                        return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The item `{}` is not found.", item))));
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
        };

        Ok(Free {
            mem,
            swap,
        })
    }
}