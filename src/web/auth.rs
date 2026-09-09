use std::fmt::Write;

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::RngExt;
use serde::{Deserialize, Serialize};

use super::{error::ApiError, state::AppState};

const COOKIE_NAME: &str = "mprober_auth";
const COOKIE_MAX_AGE: u32 = 86400;

#[derive(Debug)]
pub struct Auth {
    auth_key:      Option<String>,
    /// A random token which stands for the auth key in the cookie, so that the key itself never leaves this process.
    session_token: String,
}

impl Auth {
    pub fn new(auth_key: Option<String>) -> Self {
        let session_token = {
            let mut rng = rand::rng();

            let bytes: [u8; 32] = rng.random();

            let mut token = String::with_capacity(bytes.len() * 2);

            for byte in bytes {
                write!(&mut token, "{byte:02x}").unwrap();
            }

            token
        };

        Auth {
            auth_key,
            session_token,
        }
    }

    #[inline]
    pub fn is_required(&self) -> bool {
        self.auth_key.is_some()
    }

    #[inline]
    fn verify_key(&self, key: &str) -> bool {
        match self.auth_key.as_deref() {
            Some(auth_key) => constant_time_eq(auth_key.as_bytes(), key.as_bytes()),
            None => true,
        }
    }

    #[inline]
    fn verify_token(&self, token: &str) -> bool {
        constant_time_eq(self.session_token.as_bytes(), token.as_bytes())
    }

    /// Whether this request may read the APIs, by either the session cookie or the `Authorization` header.
    fn is_authorized(&self, headers: &HeaderMap) -> bool {
        if !self.is_required() {
            return true;
        }

        if let Some(token) = get_cookie(headers, COOKIE_NAME)
            && self.verify_token(token)
        {
            return true;
        }

        match headers.get(header::AUTHORIZATION).and_then(|value| value.to_str().ok()) {
            Some(authorization) => self.verify_key(authorization),
            None => false,
        }
    }
}

/// Compare in constant time, so that a wrong key cannot be recovered byte by byte by timing the responses.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut difference = 0u8;

    for (a, b) in a.iter().zip(b.iter()) {
        difference |= a ^ b;
    }

    difference == 0
}

fn get_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;

    cookies.split(';').find_map(|cookie| {
        let (key, value) = cookie.split_once('=')?;

        (key.trim() == name).then(|| value.trim())
    })
}

/// A reverse proxy terminates TLS itself, so the `Secure` attribute has to follow what it reports.
fn is_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|proto| proto.eq_ignore_ascii_case("https"))
}

pub async fn require_auth(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if state.auth.is_authorized(request.headers()) {
        Ok(next.run(request).await)
    } else {
        Err(ApiError::unauthorized())
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    auth_key: String,
}

#[derive(Debug, Serialize)]
pub struct AuthStatus {
    /// Whether this server was started with an auth key.
    required:      bool,
    /// Whether this request is already allowed to read the APIs.
    authenticated: bool,
}

pub async fn status(State(state): State<AppState>, headers: HeaderMap) -> Json<AuthStatus> {
    Json(AuthStatus {
        required:      state.auth.is_required(),
        authenticated: state.auth.is_authorized(&headers),
    })
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Response {
    if !state.auth.verify_key(&body.auth_key) {
        return ApiError::unauthorized().into_response();
    }

    let mut cookie = format!(
        "{COOKIE_NAME}={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={COOKIE_MAX_AGE}",
        state.auth.session_token
    );

    if is_secure(&headers) {
        cookie.push_str("; Secure");
    }

    ([(header::SET_COOKIE, cookie)], StatusCode::NO_CONTENT).into_response()
}

pub async fn logout() -> Response {
    let cookie = format!("{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");

    ([(header::SET_COOKIE, cookie)], StatusCode::NO_CONTENT).into_response()
}
