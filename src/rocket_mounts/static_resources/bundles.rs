const STATIC_RESOURCES_CACHE_MAX_AGE: u32 = 259_200;

use crate::rocket_cache_response::CacheResponse;
use crate::rocket_include_static_resources::StaticResponse;

fn static_response(id: &'static str) -> CacheResponse<StaticResponse> {
    let responder = static_response!(id);

    CacheResponse::public_only_release(responder, STATIC_RESOURCES_CACHE_MAX_AGE, false)
}

#[get("/css/bundle.min.css")]
fn css_bundle() -> CacheResponse<StaticResponse> {
    static_response("css-bundle")
}

#[get("/js/bundle.min.js")]
fn js_bundle() -> CacheResponse<StaticResponse> {
    static_response("js-bundle")
}

#[get("/css/font-roboto-mono.min.css")]
fn font_roboto_mono() -> CacheResponse<StaticResponse> {
    static_response("font-roboto-mono")
}

#[get("/fonts/RobotoMono-Bold.woff2")]
fn roboto_mono_bold() -> CacheResponse<StaticResponse> {
    static_response("RobotoMono-Bold")
}

#[get("/fonts/RobotoMono-Light.woff2")]
fn roboto_mono_light() -> CacheResponse<StaticResponse> {
    static_response("RobotoMono-Light")
}

#[get("/fonts/RobotoMono-Medium.woff2")]
fn roboto_mono_medium() -> CacheResponse<StaticResponse> {
    static_response("RobotoMono-Medium")
}

#[get("/fonts/RobotoMono-Regular.woff2")]
fn roboto_mono_regular() -> CacheResponse<StaticResponse> {
    static_response("RobotoMono-Regular")
}

#[get("/fonts/fa-solid-900.eot")]
fn fa_solid_900_eot() -> CacheResponse<StaticResponse> {
    static_response("fa-solid-900-eot")
}

#[get("/fonts/fa-solid-900.svg")]
fn fa_solid_900_svg() -> CacheResponse<StaticResponse> {
    static_response("fa-solid-900-svg")
}

#[get("/fonts/fa-solid-900.ttf")]
fn fa_solid_900_ttf() -> CacheResponse<StaticResponse> {
    static_response("fa-solid-900-ttf")
}

#[get("/fonts/fa-solid-900.woff")]
fn fa_solid_900_woff() -> CacheResponse<StaticResponse> {
    static_response("fa-solid-900-woff")
}

#[get("/fonts/fa-solid-900.woff2")]
fn fa_solid_900_woff2() -> CacheResponse<StaticResponse> {
    static_response("fa-solid-900-woff2")
}

#[get("/images/preload.svg")]
fn preload() -> CacheResponse<StaticResponse> {
    static_response("preload")
}

pub fn mounts(rocket: rocket::Rocket) -> rocket::Rocket {
    rocket
        .mount("/", routes![css_bundle, js_bundle])
        .mount("/", routes![font_roboto_mono])
        .mount("/", routes![
            roboto_mono_bold,
            roboto_mono_light,
            roboto_mono_medium,
            roboto_mono_regular
        ])
        .mount("/", routes![
            fa_solid_900_eot,
            fa_solid_900_svg,
            fa_solid_900_ttf,
            fa_solid_900_woff,
            fa_solid_900_woff2
        ])
        .mount("/", routes![preload])
}
