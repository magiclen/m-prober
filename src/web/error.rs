use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use tokio::task::JoinError;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    status:  StatusCode,
    message: String,
}

impl ApiError {
    #[inline]
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        ApiError {
            status,
            message: message.into(),
        }
    }

    #[inline]
    pub fn unauthorized() -> Self {
        ApiError::new(StatusCode::UNAUTHORIZED, "an auth key is required")
    }
}

impl From<mprober_lib::Error> for ApiError {
    #[inline]
    fn from(error: mprober_lib::Error) -> Self {
        // A missing file under `/proc` or `/sys` means the kernel does not provide the data at all, which is not a failure of this server.
        let status = if error.is_not_supported() {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        ApiError::new(status, error.to_string())
    }
}

impl From<JoinError> for ApiError {
    #[inline]
    fn from(error: JoinError) -> Self {
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
    }
}

impl IntoResponse for ApiError {
    #[inline]
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
