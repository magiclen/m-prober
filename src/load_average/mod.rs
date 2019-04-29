use std::io::{self, ErrorKind};

use crate::scanner_rust::{Scanner, ScannerError};

const LOADAVG_PATH: &'static str = "/proc/loadavg";

#[derive(Debug, Clone)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

impl LoadAverage {
    pub fn get_load_average() -> Result<LoadAverage, ScannerError> {
        let mut sc = Scanner::scan_path(LOADAVG_PATH)?;

        let one = match sc.next_f64()? {
            Some(v) => v,
            None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "Cannot find the load average within one minute.")))
        };

        let five = match sc.next_f64()? {
            Some(v) => v,
            None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "Cannot find the load average within five minutes.")))
        };

        let fifteen = match sc.next_f64()? {
            Some(v) => v,
            None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "Cannot find the load average within fifteen minutes.")))
        };

        Ok(LoadAverage {
            one,
            five,
            fifteen,
        })
    }
}