#![feature(proc_macro_hygiene, decl_macro)]
#![feature(seek_convenience)]

extern crate mprober_lib;

extern crate byte_unit;

#[macro_use]
extern crate validators;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_json;

extern crate chrono;
extern crate rand;
extern crate regex;
extern crate users;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_simple_authorization;

extern crate rocket_cache_response;
extern crate rocket_json_response;

#[macro_use]
extern crate rocket_include_static_resources;

#[macro_use]
extern crate rocket_include_handlebars;

pub mod benchmark;
pub mod rocket_mounts;

pub use mprober_lib::*;
pub use validators::number::NumberGtZero;

validated_customized_ranged_number!(pub WebMonitorInterval, u64, 1, 15);

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
