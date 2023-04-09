mod api;
mod monitor;
mod static_resources;

use std::{net::IpAddr, ops::Deref, time::Duration};

use crate::rocket::{Config as RocketConfig, Error as RocketError};

#[derive(Debug)]
struct DetectInterval(Duration);

impl DetectInterval {
    #[inline]
    fn get_value(&self) -> Duration {
        self.0
    }
}

impl Deref for DetectInterval {
    type Target = Duration;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
struct AuthKey(Option<String>);

impl AuthKey {
    #[inline]
    fn get_value(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

pub async fn launch(
    monitor: Duration,
    address: IpAddr,
    listen_port: u16,
    auth_key: Option<String>,
    only_api: bool,
) -> Result<(), RocketError> {
    let config = RocketConfig {
        address,
        port: listen_port,
        ..RocketConfig::default()
    };

    let rocket = rocket::custom(config).manage(DetectInterval(monitor)).manage(AuthKey(auth_key));

    let rocket = api::mounts(rocket);

    let rocket = if only_api {
        rocket
    } else {
        let rocket = static_resources::rocket_handler(rocket);

        monitor::rocket_handler(rocket)
    };

    let _ = rocket.launch().await?;

    Ok(())
}
