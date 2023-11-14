use std::collections::HashMap;

use rocket::{Build, Rocket, State};
use rocket_cache_response::CacheResponse;
use rocket_include_handlebars::{EtagIfNoneMatch, HandlebarsContextManager, HandlebarsResponse};
use rocket_json_response::json_gettext::JSONGetTextValue;

const HANDLEBARS_RESOURCES_CACHE_MAX_AGE: u32 = 259_200;

fn handlebars_response(responder: HandlebarsResponse) -> CacheResponse<HandlebarsResponse> {
    CacheResponse::public_only_release(responder, HANDLEBARS_RESOURCES_CACHE_MAX_AGE, false)
}

#[get("/")]
fn index(
    cm: &State<HandlebarsContextManager>,
    etag_if_none_match: &EtagIfNoneMatch,
    detect_interval: &State<super::DetectInterval>,
    auth_key: &State<super::AuthKey>,
) -> CacheResponse<HandlebarsResponse> {
    let mut map = HashMap::new();

    map.insert("version", JSONGetTextValue::Str(env!("CARGO_PKG_VERSION")));

    map.insert("timeInterval", JSONGetTextValue::from_u64(detect_interval.as_secs()));

    if let Some(auth_key) = auth_key.get_value() {
        map.insert("authKey", JSONGetTextValue::from_str(auth_key));
    }

    handlebars_response(handlebars_response!(cm, etag_if_none_match, "index", &map))
}

pub fn rocket_handler(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .attach(handlebars_resources_initializer!(
            "index" => "views/index.hbs"
        ))
        .mount("/", routes![index])
}
