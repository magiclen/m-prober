use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::path::Path;

use crate::scanner_rust::{Scanner, ScannerError};

const MOUNTS_PATH: &str = "/proc/mounts";

pub fn get_mounts() -> Result<HashMap<String, Vec<String>>, ScannerError> {
    let mut sc = Scanner::scan_path(MOUNTS_PATH)?;

    let mut mounts: HashMap<String, Vec<String>> = HashMap::with_capacity(1);

    while let Some(device_path) = sc.next()? {
        if device_path.starts_with("/dev/") {
            let device = {
                let device = &device_path[5..];

                if device.starts_with("mapper/") {
                    let device_path = Path::new(&device_path).canonicalize()?;

                    device_path.file_name().unwrap().to_string_lossy().into_owned()
                } else {
                    device.to_string()
                }
            };

            let point = match sc.next()? {
                Some(point) => point,
                None => {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        format!("The format of device `{}` is not correct.", device),
                    )))
                }
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
            return Err(ScannerError::IOError(io::Error::new(
                ErrorKind::UnexpectedEof,
                "The format of disk.mounts is not correct.".to_string(),
            )));
        }
    }

    Ok(mounts)
}
