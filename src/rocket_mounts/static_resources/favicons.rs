const STATIC_RESOURCES_CACHE_MAX_AGE: u32 = 259200;

use crate::rocket_include_static_resources::StaticResponse;
use crate::rocket_cache_response::CacheResponse;

fn static_response(id: &'static str) -> CacheResponse<StaticResponse> {
    let responder = static_response!(id);

    CacheResponse::public_only_release(responder, STATIC_RESOURCES_CACHE_MAX_AGE, false)
}

#[get("/web-app.manifest")]
fn web_app_manifest() -> CacheResponse<StaticResponse> {
    static_response("web-app.manifest")
}

#[get("/browser-config.xml")]
fn browser_config() -> CacheResponse<StaticResponse> {
    static_response("browser-config")
}

#[get("/favicon-monochrome.svg")]
fn favicon_monochrome() -> CacheResponse<StaticResponse> {
    static_response("favicon-monochrome")
}

#[get("/favicon.ico")]
fn favicon() -> CacheResponse<StaticResponse> {
    static_response("favicon")
}

#[get("/favicon-512.png")]
fn favicon_512() -> CacheResponse<StaticResponse> {
    static_response("favicon-512")
}

#[get("/favicon-192.png")]
fn favicon_192() -> CacheResponse<StaticResponse> {
    static_response("favicon-192")
}

#[get("/favicon-32.png")]
fn favicon_32() -> CacheResponse<StaticResponse> {
    static_response("favicon-32")
}

#[get("/favicon-16.png")]
fn favicon_16() -> CacheResponse<StaticResponse> {
    static_response("favicon-16")
}

#[get("/favicon-180-i.png")]
fn favicon_180_i() -> CacheResponse<StaticResponse> {
    static_response("favicon-180-i")
}

#[get("/mstile-310.png")]
fn mstile_310() -> CacheResponse<StaticResponse> {
    static_response("mstile-310")
}

#[get("/mstile-150.png")]
fn mstile_150() -> CacheResponse<StaticResponse> {
    static_response("mstile-150")
}

#[get("/mstile-70.png")]
fn mstile_70() -> CacheResponse<StaticResponse> {
    static_response("mstile-70")
}

pub fn mounts(rocket: rocket::Rocket) -> rocket::Rocket {
    rocket
        .mount("/", routes![web_app_manifest])
        .mount("/", routes![browser_config])
        .mount("/", routes![favicon_monochrome])
        .mount("/", routes![favicon])
        .mount("/", routes![favicon_512, favicon_192, favicon_32, favicon_16])
        .mount("/", routes![favicon_180_i])
        .mount("/", routes![mstile_310, mstile_150, mstile_70])
}