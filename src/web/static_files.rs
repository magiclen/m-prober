use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hasher},
    sync::LazyLock,
};

use axum::{
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use lazy_static_include::lazy_static_include_bytes;

const HTML: &str = "text/html; charset=utf-8";
const JAVASCRIPT: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const PNG: &str = "image/png";
const ICON: &str = "image/x-icon";
const MANIFEST_TYPE: &str = "application/manifest+json";

// The web UI is built with fixed file names (see `web-ui/vite.config.ts`), and the icons come from
// favicon-generator (see `web-ui/README.md`), so every asset can be listed here.
lazy_static_include_bytes! {
    INDEX_HTML => "front-end/index.html",
    BUNDLE_JS => "front-end/js/bundle.js",
    BUNDLE_CSS => "front-end/css/bundle.css",
    MANIFEST => "front-end/manifest.webmanifest",
    FAVICON => "front-end/favicon.ico",
    APPLE_TOUCH_ICON => "front-end/apple-touch-icon.png",
    ICON_192 => "front-end/icon-192.png",
    ICON_512 => "front-end/icon-512.png",
    ICON_MASK => "front-end/icon-mask.png",
}

const INDEX_PATH: &str = "/index.html";

struct StaticFile {
    content:      &'static [u8],
    content_type: &'static str,
    etag:         String,
}

impl StaticFile {
    fn new(content: &'static [u8], content_type: &'static str) -> Self {
        // The file names are fixed, so revalidation is what keeps a client from using a stale asset after an upgrade.
        let mut hasher = DefaultHasher::new();

        hasher.write(content);

        StaticFile {
            content,
            content_type,
            etag: format!("\"{:016x}\"", hasher.finish()),
        }
    }

    fn response(&self, headers: &HeaderMap) -> Response {
        let cached = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value == self.etag);

        if cached {
            return (StatusCode::NOT_MODIFIED, [(header::ETAG, self.etag.as_str())])
                .into_response();
        }

        (
            [
                (header::CONTENT_TYPE, self.content_type),
                (header::ETAG, self.etag.as_str()),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            self.content,
        )
            .into_response()
    }
}

static FILES: LazyLock<HashMap<&'static str, StaticFile>> = LazyLock::new(|| {
    [
        (INDEX_PATH, *INDEX_HTML, HTML),
        ("/js/bundle.js", *BUNDLE_JS, JAVASCRIPT),
        ("/css/bundle.css", *BUNDLE_CSS, CSS),
        ("/manifest.webmanifest", *MANIFEST, MANIFEST_TYPE),
        ("/favicon.ico", *FAVICON, ICON),
        ("/apple-touch-icon.png", *APPLE_TOUCH_ICON, PNG),
        ("/icon-192.png", *ICON_192, PNG),
        ("/icon-512.png", *ICON_512, PNG),
        ("/icon-mask.png", *ICON_MASK, PNG),
    ]
    .into_iter()
    .map(|(path, content, content_type)| (path, StaticFile::new(content, content_type)))
    .collect()
});

/// Serve an asset, falling back to the page itself so that the single-page app owns every other path.
pub async fn serve(uri: Uri, headers: HeaderMap) -> Response {
    let file = FILES.get(uri.path()).unwrap_or_else(|| &FILES[INDEX_PATH]);

    file.response(&headers)
}
