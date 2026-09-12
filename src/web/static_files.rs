use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hasher},
    io::Write,
    sync::LazyLock,
};

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use flate2::{Compression, write::GzEncoder};
use lazy_static_include::lazy_static_include_bytes;

const HTML: &str = "text/html; charset=utf-8";
const JAVASCRIPT: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const PNG: &str = "image/png";
const ICON: &str = "image/x-icon";
const MANIFEST_TYPE: &str = "application/manifest+json";
const WOFF2: &str = "font/woff2";

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
    FONT_LATIN => "front-end/fonts/roboto-mono-latin-wght-normal.woff2",
    FONT_LATIN_EXT => "front-end/fonts/roboto-mono-latin-ext-wght-normal.woff2",
    FONT_CYRILLIC => "front-end/fonts/roboto-mono-cyrillic-wght-normal.woff2",
    FONT_CYRILLIC_EXT => "front-end/fonts/roboto-mono-cyrillic-ext-wght-normal.woff2",
    FONT_GREEK => "front-end/fonts/roboto-mono-greek-wght-normal.woff2",
    FONT_VIETNAMESE => "front-end/fonts/roboto-mono-vietnamese-wght-normal.woff2",
}

const INDEX_PATH: &str = "/index.html";

struct StaticFile {
    content:      &'static [u8],
    content_type: &'static str,
    etag:         String,
    /// The gzip of `content` and the tag that names that representation, since it is a different one and must not share a tag with the original.
    gzipped:      Option<(Vec<u8>, String)>,
}

impl StaticFile {
    fn new(content: &'static [u8], content_type: &'static str) -> Self {
        // The file names are fixed, so revalidation is what keeps a client from using a stale asset after an upgrade.
        let mut hasher = DefaultHasher::new();

        hasher.write(content);

        let hash = hasher.finish();

        StaticFile {
            gzipped: gzip(content, content_type).map(|body| (body, format!("\"{hash:016x}-gz\""))),
            content,
            content_type,
            etag: format!("\"{hash:016x}\""),
        }
    }

    fn response(&'static self, headers: &HeaderMap) -> Response {
        let gzipped = self.gzipped.as_ref().filter(|_| accepts_gzip(headers));

        let (content, etag) = match gzipped {
            Some((content, etag)) => (content.as_slice(), etag.as_str()),
            None => (self.content, self.etag.as_str()),
        };

        let cached = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value == etag);

        // A cache in the middle would otherwise hand the compressed body to a client which cannot read it.
        let vary = (header::VARY, "accept-encoding");

        if cached {
            return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag), vary]).into_response();
        }

        let mut response = (
            [
                (header::CONTENT_TYPE, self.content_type),
                (header::ETAG, etag),
                (header::CACHE_CONTROL, "no-cache"),
                vary,
            ],
            content,
        )
            .into_response();

        if gzipped.is_some() {
            response
                .headers_mut()
                .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        }

        response
    }
}

/// Compress an asset once, so that a page fetched over a slow link is not held up by a third of a megabyte of JavaScript.
///
/// `None` for a type that is already compressed, e.g. the icons and the fonts, and for the rare file that gzip cannot make any smaller.
fn gzip(content: &'static [u8], content_type: &'static str) -> Option<Vec<u8>> {
    if !matches!(content_type, HTML | JAVASCRIPT | CSS | MANIFEST_TYPE) {
        return None;
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());

    encoder.write_all(content).ok()?;

    let compressed = encoder.finish().ok()?;

    (compressed.len() < content.len()).then_some(compressed)
}

/// Whether the client takes gzip. Naming it with `q=0` says the opposite, so finding the name alone is not enough.
fn accepts_gzip(headers: &HeaderMap) -> bool {
    let Some(accepted) = headers.get(header::ACCEPT_ENCODING).and_then(|value| value.to_str().ok())
    else {
        return false;
    };

    accepted.split(',').any(|encoding| {
        let mut parts = encoding.split(';').map(str::trim);

        let Some(name) = parts.next() else {
            return false;
        };

        if !name.eq_ignore_ascii_case("gzip") && name != "*" {
            return false;
        }

        !parts.any(|parameter| {
            parameter
                .strip_prefix("q=")
                .is_some_and(|quality| quality.parse::<f32>().is_ok_and(|quality| quality <= 0f32))
        })
    })
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
        ("/fonts/roboto-mono-latin-wght-normal.woff2", *FONT_LATIN, WOFF2),
        ("/fonts/roboto-mono-latin-ext-wght-normal.woff2", *FONT_LATIN_EXT, WOFF2),
        ("/fonts/roboto-mono-cyrillic-wght-normal.woff2", *FONT_CYRILLIC, WOFF2),
        ("/fonts/roboto-mono-cyrillic-ext-wght-normal.woff2", *FONT_CYRILLIC_EXT, WOFF2),
        ("/fonts/roboto-mono-greek-wght-normal.woff2", *FONT_GREEK, WOFF2),
        ("/fonts/roboto-mono-vietnamese-wght-normal.woff2", *FONT_VIETNAMESE, WOFF2),
    ]
    .into_iter()
    .map(|(path, content, content_type)| (path, StaticFile::new(content, content_type)))
    .collect()
});

/// Compress every asset now, so that the first visitor does not wait for it and no runtime thread is held up doing it.
pub fn warm_up() {
    LazyLock::force(&FILES);
}

/// Serve an asset, falling back to the page itself so that the single-page app owns every other path.
pub async fn serve(uri: Uri, headers: HeaderMap) -> Response {
    let file = FILES.get(uri.path()).unwrap_or_else(|| &FILES[INDEX_PATH]);

    file.response(&headers)
}
