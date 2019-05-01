use std::collections::HashMap;

use crate::rocket::Rocket;

use crate::rocket_cache_response::CacheResponse;
use crate::rocket_include_handlebars::{EtagIfNoneMatch, HandlebarsResponse};
use crate::rocket_json_response::{json_gettext::JSONGetTextValue};

const HANDLEBARS_RESOURCES_CACHE_MAX_AGE: u32 = 259200;

handlebars_resources_initialize!(
    "index", "views/index.hbs",
);

fn handlebars_response(responder: HandlebarsResponse) -> CacheResponse<HandlebarsResponse> {
    CacheResponse::public_only_release(responder, HANDLEBARS_RESOURCES_CACHE_MAX_AGE, false)
}

#[get("/")]
fn index(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<HandlebarsResponse> {
    let mut map = HashMap::new();

    map.insert("version", JSONGetTextValue::Str(crate::CARGO_PKG_VERSION));

    handlebars_response(handlebars_response!(etag_if_none_match, "index", &map))
}

pub fn mounts(rocket: Rocket) -> Rocket {
    rocket
        .mount("/", routes![index])
}