#![feature(duration_float)]

extern crate scanner_rust;

use std::time::{Duration, SystemTime};
use std::io::{self, ErrorKind};

use scanner_rust::{Scanner, ScannerError};

const UPTIME_PATH: &'static str = "/proc/uptime";

const RTC_PATH: &'static str = "/proc/driver/rtc";
const ITEMS: [&'static str; 2] = ["rtc_time", "rtc_date"];

pub fn get_uptime() -> Result<Duration, ScannerError> {
    let mut sc = Scanner::scan_path(UPTIME_PATH)?;

    let uptime = match sc.next_f64()? {
        Some(v) => v,
        None => return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, "Cannot find the uptime.")))
    };

    Ok(Duration::from_secs_f64(uptime))
}

pub fn get_system_time() -> SystemTime {
    SystemTime::now()
}

#[derive(Debug, Clone)]
pub struct RTCDateTime {
    pub rtc_time: String,
    pub rtc_date: String,
}

impl RTCDateTime {
    pub fn get_rtc_date_time() -> Result<RTCDateTime, ScannerError> {
        let mut sc = Scanner::scan_path(RTC_PATH)?;

        let mut item_values: Vec<String> = Vec::with_capacity(ITEMS.len());

        for &item in ITEMS.iter() {
            let item_len = item.len();

            loop {
                let line = sc.next_line()?;

                match line {
                    Some(line) => {
                        if line.as_str().starts_with(item) {
                            match line[item_len..].find(":") {
                                Some(colon_index) => {
                                    let item = line[(item_len + colon_index + 1)..].trim().to_string();

                                    item_values.push(item);
                                }
                                None => {
                                    return Err(ScannerError::IOError(io::Error::new(ErrorKind::InvalidInput, format!("The item `{}` has no colon.", item))));
                                }
                            }

                            break;
                        }
                    }
                    None => {
                        return Err(ScannerError::IOError(io::Error::new(ErrorKind::UnexpectedEof, format!("The item `{}` is not found.", item))));
                    }
                }
            }
        }

        let rtc_date = item_values.pop().unwrap();
        let rtc_time = item_values.pop().unwrap();

        Ok(RTCDateTime {
            rtc_time,
            rtc_date,
        })
    }
}