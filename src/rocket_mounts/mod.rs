mod api;
mod bundles;
mod monitor;

use std::time::Duration;

use crate::rocket::{Config, config::Environment};

use crate::rand::{self, RngCore};
use crate::base64;

static mut AUTH_KEY: Option<String> = None;
static mut DETECT_INTERVAL: Duration = Duration::from_secs(0);

pub fn launch(monitor: Duration, port: u16, auth_key: Option<String>, only_api: bool) {
    unsafe {
        DETECT_INTERVAL = monitor;
    }

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

    let rocket = api::mounts(rocket);

    let rocket = if only_api {
        rocket
    } else {
        let rocket = bundles::rocket_handler(rocket);

        let rocket = monitor::rocket_handler(rocket);

        rocket
    };

    rocket.launch();
}