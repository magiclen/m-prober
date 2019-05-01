use std::collections::HashMap;
use std::io::{self, ErrorKind};

use crate::scanner_rust::{Scanner, ScannerError};

const MOUNTS_PATH: &'static str = "/proc/mounts";

pub fn get_mounts() -> Result<HashMap<String, Vec<String>>, ScannerError> {
    let mut sc = Scanner::scan_path(MOUNTS_PATH)?;

    let mut mounts: HashMap<String, Vec<String>> = HashMap::with_capacity(1);

    loop {
        let device_path = match sc.next()? {
            Some(device_path) => device_path,
            None => break
        };

        if device_path.starts_with("/dev/") {
            let device = device_path[5..].to_string();

            let point = match sc.next()? {
                Some(point) => point,
                None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The format of device `{}` is not correct.", device))))
            };

            match mounts.get_mut(&device) {
                Some(devices) => {
                    devices.push(point);
                }
                None => {
                    mounts.insert(device, vec![point]);
                }
            }
        }

        if sc.next_line()?.is_none() {
            return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "The format of disk.mounts is not correct.".to_string())));
        }
    }

    Ok(mounts)
}