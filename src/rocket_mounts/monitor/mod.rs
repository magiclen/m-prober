use std::collections::HashMap;

use crate::rocket::{Rocket, State};

use crate::rocket_cache_response::CacheResponse;
use crate::rocket_include_handlebars::{EtagIfNoneMatch, HandlebarsResponse};
use crate::rocket_json_response::{json_gettext::JSONGetTextValue};

const HANDLEBARS_RESOURCES_CACHE_MAX_AGE: u32 = 259200;

fn handlebars_response(responder: HandlebarsResponse) -> CacheResponse<HandlebarsResponse> {
    CacheResponse::public_only_release(responder, HANDLEBARS_RESOURCES_CACHE_MAX_AGE, false)
}

#[get("/")]
fn index(etag_if_none_match: EtagIfNoneMatch, detect_interval: State<super::DetectInterval>, auth_key: State<super::AuthKey>) -> CacheResponse<HandlebarsResponse> {
    let mut map = HashMap::new();

    map.insert("version", JSONGetTextValue::Str(crate::CARGO_PKG_VERSION));

    map.insert("timeInterval", JSONGetTextValue::from_u64(detect_interval.as_secs()));

    if let Some(auth_key) = auth_key.get_value() {
        map.insert("authKey", JSONGetTextValue::from_str(auth_key));
    }

    handlebars_response(handlebars_response!(etag_if_none_match, "index", &map))
}

pub fn rocket_handler(rocket: Rocket) -> Rocket {
    rocket
        .attach(HandlebarsResponse::fairing(|handlebars| {
            handlebars_resources_initialize!(
                handlebars,
                "index", "views/index.hbs",
            );
        }))
        .mount("/", routes![index])
}