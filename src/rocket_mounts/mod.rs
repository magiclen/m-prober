mod api;

use std::time::Duration;

use crate::rocket::{Config, config::Environment};
use crate::rand::{self, RngCore};
use crate::base64;

const MAX_DETECT_INTERVAL: u64 = 5000;

static mut AUTH_KEY: Option<String> = None;

pub fn launch(monitor: Duration, port: u16, auth_key: Option<String>) {
    unsafe {
        AUTH_KEY = auth_key;
    }

    let mut config = Config::build(if cfg!(debug_assertions) {
        Environment::Development
    } else {
        Environment::Production
    });

    let mut secret_key = [0u8; 32];

    rand::thread_rng().fill_bytes(&mut secret_key);

    config.secret_key = Some(base64::encode(&secret_key));

    config.port = port;

    let rocket = rocket::custom(config.unwrap());

    let rocket = rocket.manage(monitor);

    let rocket = api::mounts(rocket);

    rocket.launch();
}