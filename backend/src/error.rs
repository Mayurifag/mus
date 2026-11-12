use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;

/// An HTTP error whose display text is sent to the client as JSON `detail`.
///
/// Converting an internal error preserves its cause but also exposes its outer
/// display message. Add safe context before conversion when the cause is sensitive.
///
/// ```
/// use mus_backend::error::AppError;
/// let error = AppError::bad_request("Filename too long");
/// assert_eq!(error.to_string(), "Filename too long");
/// assert_eq!(error.status.as_u16(), 400);
/// ```
#[derive(Debug)]
pub struct AppError {
    /// HTTP response status.
    pub status: StatusCode,
    /// Client-visible error message; do not store secrets here.
    pub message: String,
    cause: Option<anyhow::Error>,
}

impl AppError {
    fn new(status: StatusCode, message: String) -> Self {
        Self {
            status,
            message,
            cause: None,
        }
    }

    /// Creates a 400 response without an internal cause.
    pub fn bad_request(message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message.into())
    }

    /// Creates a 403 response without an internal cause.
    pub fn forbidden(message: &str) -> Self {
        Self::new(StatusCode::FORBIDDEN, message.into())
    }

    /// Creates a 409 response without an internal cause.
    pub fn conflict(message: &str) -> Self {
        Self::new(StatusCode::CONFLICT, message.into())
    }

    /// Creates a 404 response without an internal cause.
    pub fn not_found(message: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, message.into())
    }

    /// Creates a 429 response without an internal cause.
    pub fn too_many(message: &str) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, message.into())
    }

    /// Creates a 413 response without an internal cause.
    pub fn payload_too_large(message: String) -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_ref().map(|error| error.as_ref())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"detail": self.message}))).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
            cause: Some(error),
        }
    }
}

macro_rules! internal_error_from {
    ($error_type:ty) => {
        impl From<$error_type> for AppError {
            fn from(error: $error_type) -> Self {
                anyhow::Error::new(error).into()
            }
        }
    };
}

internal_error_from!(std::io::Error);
internal_error_from!(rusqlite::Error);
internal_error_from!(serde_json::Error);
internal_error_from!(axum::extract::multipart::MultipartError);

#[cfg(test)]
mod tests {
    use super::AppError;
    use std::{error::Error, io};

    #[test]
    fn internal_errors_preserve_their_cause_and_public_message() {
        let cause = io::Error::new(io::ErrorKind::PermissionDenied, "cannot write music");
        let error = AppError::from(cause);
        assert_eq!(error.to_string(), "cannot write music");
        assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let cause = error.source().unwrap().downcast_ref::<io::Error>().unwrap();
        assert_eq!(cause.kind(), io::ErrorKind::PermissionDenied);
    }
}
