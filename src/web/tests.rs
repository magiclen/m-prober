use std::time::Duration;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

use super::{
    router,
    state::{AppState, Shutdown},
};

const TEST_DETECT_INTERVAL: Duration = Duration::from_millis(1000);
const TEST_AUTH_KEY: &str = "magic";

fn create_router(auth_key: Option<&str>) -> Router {
    build_router(auth_key, true)
}

/// The web page is only served when it was not turned off, so its own tests have to ask for it.
fn create_page_router() -> Router {
    build_router(None, false)
}

fn build_router(auth_key: Option<&str>, only_api: bool) -> Router {
    let (_sender, shutdown) = Shutdown::channel();

    router(AppState::new(TEST_DETECT_INTERVAL, auth_key.map(String::from), shutdown), only_api)
}

async fn status_of(router: Router, request: Request<Body>) -> StatusCode {
    router.oneshot(request).await.unwrap().status()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn login(auth_key: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/auth")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!("{{\"auth_key\":\"{auth_key}\"}}")))
        .unwrap()
}

#[tokio::test]
async fn test_no_need_auth() {
    let router = create_router(None);

    let with_header = Request::builder()
        .uri("/api/hostname")
        .header(header::AUTHORIZATION, TEST_AUTH_KEY)
        .body(Body::empty())
        .unwrap();

    assert_eq!(StatusCode::OK, status_of(router.clone(), with_header).await);
    assert_eq!(StatusCode::OK, status_of(router, get("/api/hostname")).await);
}

#[tokio::test]
async fn test_need_auth() {
    let router = create_router(Some(TEST_AUTH_KEY));

    let authorized = Request::builder()
        .uri("/api/hostname")
        .header(header::AUTHORIZATION, TEST_AUTH_KEY)
        .body(Body::empty())
        .unwrap();

    assert_eq!(StatusCode::OK, status_of(router.clone(), authorized).await);
    assert_eq!(StatusCode::UNAUTHORIZED, status_of(router, get("/api/hostname")).await);
}

#[tokio::test]
async fn test_login_sets_a_working_cookie() {
    let router = create_router(Some(TEST_AUTH_KEY));

    let response = router.clone().oneshot(login(TEST_AUTH_KEY)).await.unwrap();

    assert_eq!(StatusCode::NO_CONTENT, response.status());

    let cookie = response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
    let cookie = cookie.split(';').next().unwrap().to_string();

    let with_cookie = Request::builder()
        .uri("/api/hostname")
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();

    assert_eq!(StatusCode::OK, status_of(router, with_cookie).await);
}

#[tokio::test]
async fn test_login_rejects_a_wrong_key() {
    let router = create_router(Some(TEST_AUTH_KEY));

    assert_eq!(StatusCode::UNAUTHORIZED, status_of(router, login("wrong")).await);
}

#[tokio::test]
async fn test_config_stays_public() {
    let router = create_router(Some(TEST_AUTH_KEY));

    assert_eq!(StatusCode::OK, status_of(router, get("/api/config")).await);
}

#[tokio::test]
async fn test_the_bundle_is_served_compressed() {
    let router = create_page_router();

    let plain = router.clone().oneshot(get("/js/bundle.js")).await.unwrap();

    assert_eq!(StatusCode::OK, plain.status());
    assert_eq!(None, plain.headers().get(header::CONTENT_ENCODING));

    let asking_for_gzip = Request::builder()
        .uri("/js/bundle.js")
        .header(header::ACCEPT_ENCODING, "gzip, deflate, br")
        .body(Body::empty())
        .unwrap();

    let compressed = router.oneshot(asking_for_gzip).await.unwrap();

    assert_eq!(StatusCode::OK, compressed.status());
    assert_eq!("gzip", compressed.headers().get(header::CONTENT_ENCODING).unwrap());
    assert_eq!("accept-encoding", compressed.headers().get(header::VARY).unwrap());

    let plain = to_bytes(plain.into_body(), usize::MAX).await.unwrap();
    let compressed = to_bytes(compressed.into_body(), usize::MAX).await.unwrap();

    assert!(compressed.len() < plain.len());
}

#[tokio::test]
async fn test_a_client_which_refuses_gzip_gets_the_original() {
    let router = create_page_router();

    let refusing_gzip = Request::builder()
        .uri("/js/bundle.js")
        .header(header::ACCEPT_ENCODING, "gzip;q=0")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(refusing_gzip).await.unwrap();

    assert_eq!(StatusCode::OK, response.status());
    assert_eq!(None, response.headers().get(header::CONTENT_ENCODING));
}
