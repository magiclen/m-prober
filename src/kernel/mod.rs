use std::io::{self, ErrorKind};

use crate::scanner_rust::{Scanner, ScannerError};

const KERNEL_VERSION_PATH: &'static str = "/proc/version";

#[inline]
pub fn get_kernel_version() -> Result<String, ScannerError> {
    let mut sc = Scanner::scan_path(KERNEL_VERSION_PATH)?;

    match sc.next()? {
        Some(linux) => {
            if linux.eq("Linux") {
                match sc.next()? {
                    Some(version) => {
                        if version.eq("version") {
                            match sc.next()? {
                                Some(kernel_version) => Ok(kernel_version),
                                None => Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "Cannot find the kernel version.".to_string())))
                            }
                        } else {
                            Err(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The first token is not `version`.".to_string())))
                        }
                    }
                    None => {
                        Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "Cannot find the token `version`.".to_string())))
                    }
                }
            } else {
                Err(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, "The first token is not `Linux`.".to_string())))
            }
        }
        None => {
            Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "Cannot find the token `Linux`.".to_string())))
        }
    }
}