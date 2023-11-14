mod api;
mod monitor;
mod static_resources;

use std::{net::IpAddr, ops::Deref, time::Duration};

use rocket::{Build, Config, Rocket};

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

pub fn create(
    monitor: Duration,
    address: IpAddr,
    listen_port: u16,
    auth_key: Option<String>,
    only_api: bool,
) -> Rocket<Build> {
    let figment = Config::figment().merge(("address", address)).merge(("port", listen_port));

    let rocket = rocket::custom(figment).manage(DetectInterval(monitor)).manage(AuthKey(auth_key));

    let rocket = api::mounts(rocket);

    if only_api {
        rocket
    } else {
        let rocket = static_resources::rocket_handler(rocket);

        monitor::rocket_handler(rocket)
    }
}
