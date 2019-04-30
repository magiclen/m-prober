use std::time::Duration;

use crate::rocket::{Config, config::Environment};
use crate::rand::{self, RngCore};
use crate::base64;

pub fn launch(monitor: Duration, port: u16) {
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

    rocket.launch();
}