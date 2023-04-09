#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_include_static_resources;

#[macro_use]
extern crate rocket_include_handlebars;

pub mod benchmark;
pub mod rocket_mounts;

pub use mprober_lib::*;
use validators::prelude::*;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Validator)]
#[validator(number(nan(NotAllow), range(Inside(min = 1))))]
pub struct MonitorInterval(f64);

impl MonitorInterval {
    #[inline]
    pub fn get_number(&self) -> f64 {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Validator)]
#[validator(unsigned_integer(range(Inside(min = 1, max = 15))))]
pub struct WebMonitorInterval(u64);

impl WebMonitorInterval {
    #[inline]
    pub fn get_number(&self) -> u64 {
        self.0
    }
}

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
