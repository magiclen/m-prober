use crate::validators::ValidatedCustomizedNumberError;

use crate::rocket::{Rocket, request::LenientForm, http::Status};
use crate::rocket_simple_authorization::SimpleAuthorization;
use crate::rocket_cache_response::CacheResponse;
use crate::rocket_json_response::{JSONResponse, json_gettext::JSONGetTextValue};

pub struct Auth;

impl SimpleAuthorization for Auth {
    fn has_authority<S: AsRef<str>>(key: Option<S>) -> bool {
        match unsafe { super::AUTH_KEY.as_ref() } {
            Some(auth_key) => {
                match key {
                    Some(key) => key.as_ref().eq(auth_key),
                    None => false
                }
            }
            None => true
        }
    }

    fn create_auth<S: Into<String>>(_key: Option<S>) -> Auth {
        Auth
    }
}

authorizer!(Auth);

validated_customized_ranged_number!(Interval, u64, 1, super::MAX_DETECT_INTERVAL);

#[derive(FromForm)]
struct MonitorGetModel {
    interval: Option<Result<Interval, ValidatedCustomizedNumberError>>
}

#[get("/monitor?<model..>")]
fn monitor(_auth: Auth, model: LenientForm<MonitorGetModel>) -> CacheResponse<JSONResponse<'static>> {
    drop(model);
    CacheResponse::NoStore(JSONResponse::ok(JSONGetTextValue::Str("Test")))
}

#[get("/monitor", rank = 2)]
fn monitor_401() -> Status {
    Status::Unauthorized
}

pub fn mounts(rocket: Rocket) -> Rocket {
    rocket
        .mount("/api", routes![monitor, monitor_401])
}