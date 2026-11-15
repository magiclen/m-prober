use std::{io::Read, time::Duration};

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
    build_router(auth_key, true, "/")
}

/// The web page is only served when it was not turned off, so its own tests have to ask for it.
fn create_page_router() -> Router {
    build_router(None, false, "/")
}

fn build_router(auth_key: Option<&str>, only_api: bool, base_path: &str) -> Router {
    let (_sender, shutdown) = Shutdown::channel();

    router(
        AppState::new(TEST_DETECT_INTERVAL, auth_key.map(String::from), base_path, shutdown),
        only_api,
    )
}

async fn status_of(router: Router, request: Request<Body>) -> StatusCode {
    router.oneshot(request).await.unwrap().status()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn login(auth_key: &str, base_path: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("{}/api/auth", base_path.trim_end_matches('/')))
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
    for base_path in ["/", "/admin/tools/mprober"] {
        let router = build_router(Some(TEST_AUTH_KEY), false, base_path);
        let prefix = base_path.trim_end_matches('/');

        assert_eq!(
            StatusCode::OK,
            status_of(router.clone(), get(&format!("{prefix}/api/config"))).await
        );
        assert_eq!(
            StatusCode::UNAUTHORIZED,
            status_of(router.clone(), get(&format!("{prefix}/api/hostname"))).await
        );

        let mut request = login(TEST_AUTH_KEY, base_path);
        request.headers_mut().insert("x-forwarded-proto", "https".parse().unwrap());
        let response = router.clone().oneshot(request).await.unwrap();

        assert_eq!(StatusCode::NO_CONTENT, response.status());

        let cookie = response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.split("; ").any(|part| part == format!("Path={base_path}")));
        assert!(cookie.ends_with("; Secure"));
        let cookie = cookie.split(';').next().unwrap().to_string();

        for (path, content_type) in
            [("hostname", "application/json"), ("all/stream", "text/event-stream")]
        {
            let with_cookie = Request::builder()
                .uri(format!("{prefix}/api/{path}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap();
            let response = router.clone().oneshot(with_cookie).await.unwrap();

            assert_eq!(StatusCode::OK, response.status());
            assert_eq!(content_type, response.headers().get(header::CONTENT_TYPE).unwrap());
        }

        let request = Request::builder()
            .method("POST")
            .uri(format!("{prefix}/api/logout"))
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();

        assert_eq!(StatusCode::NO_CONTENT, response.status());
        let cookie = response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.split("; ").any(|part| part == format!("Path={base_path}")));
        assert!(cookie.ends_with("Max-Age=0"));
    }
}

#[tokio::test]
async fn test_login_rejects_a_wrong_key() {
    let router = create_router(Some(TEST_AUTH_KEY));

    assert_eq!(StatusCode::UNAUTHORIZED, status_of(router, login("wrong", "/")).await);
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

#[tokio::test]
async fn test_the_page_and_assets_follow_the_base_path() {
    let mut tags = Vec::new();

    for base_path in ["/", "/mprober", "/admin/tools/mprober"] {
        let router = build_router(None, false, base_path);
        let prefix = base_path.trim_end_matches('/');
        let expected_base = format!("<base href=\"{prefix}/\"");

        for path in [base_path.to_string(), format!("{prefix}/"), format!("{prefix}/nested/page")] {
            let response = router.clone().oneshot(get(&path)).await.unwrap();
            assert_eq!(StatusCode::OK, response.status());
            assert_eq!(
                "text/html; charset=utf-8",
                response.headers().get(header::CONTENT_TYPE).unwrap()
            );
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert!(std::str::from_utf8(&body).unwrap().contains(&expected_base));
        }

        for (path, content_type) in [
            ("js/bundle.js", "text/javascript; charset=utf-8"),
            ("css/bundle.css", "text/css; charset=utf-8"),
            ("fonts/roboto-mono-latin-wght-normal.woff2", "font/woff2"),
            ("favicon.ico", "image/x-icon"),
            ("icon-192.png", "image/png"),
            ("manifest.webmanifest", "application/manifest+json"),
        ] {
            let response = router.clone().oneshot(get(&format!("{prefix}/{path}"))).await.unwrap();
            assert_eq!(StatusCode::OK, response.status());
            assert_eq!(content_type, response.headers().get(header::CONTENT_TYPE).unwrap());
        }

        let request = Request::builder()
            .uri(format!("{prefix}/"))
            .header(header::ACCEPT_ENCODING, "gzip")
            .body(Body::empty())
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!("gzip", response.headers().get(header::CONTENT_ENCODING).unwrap());
        let tag = response.headers().get(header::ETAG).unwrap().clone();
        assert!(!tags.contains(&tag));
        tags.push(tag.clone());
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let mut html = String::new();
        flate2::read::GzDecoder::new(body.as_ref()).read_to_string(&mut html).unwrap();
        assert!(html.contains(&expected_base));

        let request = Request::builder()
            .uri(format!("{prefix}/"))
            .header(header::ACCEPT_ENCODING, "gzip")
            .header(header::IF_NONE_MATCH, tag)
            .body(Body::empty())
            .unwrap();
        assert_eq!(StatusCode::NOT_MODIFIED, status_of(router.clone(), request).await);

        if base_path != "/" {
            for path in ["/", "/api/config", "/js/bundle.js", &format!("{prefix}-other")] {
                assert_eq!(StatusCode::NOT_FOUND, status_of(router.clone(), get(path)).await);
            }
        }
    }
}

#[tokio::test]
async fn test_only_api_respects_the_base_path() {
    for base_path in ["/", "/admin/tools/mprober"] {
        let router = build_router(None, true, base_path);
        let prefix = base_path.trim_end_matches('/');

        assert_eq!(
            StatusCode::OK,
            status_of(router.clone(), get(&format!("{prefix}/api/config"))).await
        );

        for path in [format!("{prefix}/"), format!("{prefix}/js/bundle.js")] {
            assert_eq!(StatusCode::NOT_FOUND, status_of(router.clone(), get(&path)).await);
        }
    }
}
