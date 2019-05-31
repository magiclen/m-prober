const STATIC_RESOURCES_CACHE_MAX_AGE: u32 = 259200;

use crate::rocket_include_static_resources::{EtagIfNoneMatch, StaticResponse};
use crate::rocket_cache_response::CacheResponse;

fn static_response(etag_if_none_match: EtagIfNoneMatch, id: &'static str) -> CacheResponse<StaticResponse> {
    let responder = static_response!(etag_if_none_match, id);

    CacheResponse::public_only_release(responder, STATIC_RESOURCES_CACHE_MAX_AGE, false)
}

#[get("/web-app.manifest")]
fn web_app_manifest(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "web-app.manifest")
}

#[get("/browser-config.xml")]
fn browser_config(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "browser-config")
}

#[get("/favicon-monochrome.svg")]
fn favicon_monochrome(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "favicon-monochrome")
}

#[get("/favicon.ico")]
fn favicon(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "favicon")
}

#[get("/favicon-512.png")]
fn favicon_512(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "favicon-512")
}

#[get("/favicon-192.png")]
fn favicon_192(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "favicon-192")
}

#[get("/favicon-32.png")]
fn favicon_32(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "favicon-32")
}

#[get("/favicon-16.png")]
fn favicon_16(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "favicon-16")
}

#[get("/favicon-180-i.png")]
fn favicon_180_i(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "favicon-180-i")
}

#[get("/mstile-310.png")]
fn mstile_310(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "mstile-310")
}

#[get("/mstile-150.png")]
fn mstile_150(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "mstile-150")
}

#[get("/mstile-70.png")]
fn mstile_70(etag_if_none_match: EtagIfNoneMatch) -> CacheResponse<StaticResponse> {
    static_response(etag_if_none_match, "mstile-70")
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