use std::fmt::Write;
use std::io::{self, ErrorKind};
use std::time::Duration;

use crate::chrono::prelude::*;
use crate::scanner_rust::{Scanner, ScannerError};

const UPTIME_PATH: &str = "/proc/uptime";
const STAT_PATH: &str = "/proc/stat";

const RTC_PATH: &str = "/proc/driver/rtc";
const ITEMS: [&str; 2] = ["rtc_time", "rtc_date"];

pub fn get_uptime() -> Result<Duration, ScannerError> {
    let mut sc = Scanner::scan_path(UPTIME_PATH)?;

    let uptime = match sc.next_f64()? {
        Some(v) => v,
        None => {
            return Err(ScannerError::IOError(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Cannot find the uptime.",
            )))
        }
    };

    Ok(Duration::from_secs_f64(uptime))
}

pub fn format_duration(duration: Duration) -> String {
    let sec = duration.as_secs();

    let days = sec / 86400;

    let sec = sec % 86400;

    let hours = sec / 3600;

    let sec = sec % 3600;

    let minutes = sec / 60;

    let seconds = sec % 60;

    let mut s = String::new();

    if days > 0 {
        s.write_fmt(format_args!("{} day", days)).unwrap();

        if days > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if hours > 0 || (days > 0) && (minutes > 0 || seconds > 0) {
        s.write_fmt(format_args!("{} hour", hours)).unwrap();

        if hours > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if minutes > 0 || (hours > 0 && seconds > 0) {
        s.write_fmt(format_args!("{} minute", minutes)).unwrap();

        if minutes > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if seconds > 0 {
        s.write_fmt(format_args!("{} second", seconds)).unwrap();

        if seconds > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    debug_assert!(s.len() >= 2);

    if let Some(index) = s.as_str()[..(s.len() - 2)].rfind(", ") {
        s.insert_str(index + 2, "and ");
    }

    let len = s.len();

    let mut v = s.into_bytes();

    unsafe {
        v.set_len(len - 2);

        String::from_utf8_unchecked(v)
    }
}

pub fn get_btime() -> Result<DateTime<Utc>, ScannerError> {
    let mut sc = Scanner::scan_path(STAT_PATH)?;

    loop {
        let label = sc.next()?;

        match label {
            Some(label) => {
                if label.as_str().eq("btime") {
                    match sc.next_u64()? {
                        Some(btime) => {
                            return Ok(DateTime::from_utc(
                                NaiveDateTime::from_timestamp(btime as i64, 0),
                                Utc,
                            ))
                        }
                        None => {
                            return Err(ScannerError::IOError(io::Error::new(
                                ErrorKind::UnexpectedEof,
                                "The format of item `btime` is correct.",
                            )));
                        }
                    }
                } else if sc.drop_next_line()?.is_none() {
                    return Err(ScannerError::IOError(io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "The format of item `btime` is correct.",
                    )));
                }
            }
            None => {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "The item `btime` is not found.",
                )));
            }
        }
    }
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
                match sc.next_line()? {
                    Some(line) => {
                        if line.starts_with(item) {
                            match line[item_len..].find(':') {
                                Some(colon_index) => {
                                    let item =
                                        line[(item_len + colon_index + 1)..].trim().to_string();

                                    item_values.push(item);
                                }
                                None => {
                                    return Err(ScannerError::IOError(io::Error::new(
                                        ErrorKind::InvalidInput,
                                        format!("The item `{}` has no colon.", item),
                                    )));
                                }
                            }

                            break;
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

        let rtc_date = item_values.pop().unwrap();
        let rtc_time = item_values.pop().unwrap();

        Ok(RTCDateTime {
            rtc_time,
            rtc_date,
        })
    }
}
