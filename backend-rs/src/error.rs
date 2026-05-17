use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl AppError {
    pub fn bad_request(message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn forbidden(message: &str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }

    pub fn conflict(message: &str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    pub fn not_found(message: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn too_many(message: &str) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: message.into(),
        }
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
        }
    }
}

macro_rules! internal_error_from {
    ($error_type:ty) => {
        impl From<$error_type> for AppError {
            fn from(error: $error_type) -> Self {
                Self {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: error.to_string(),
                }
            }
        }
    };
}

internal_error_from!(std::io::Error);
internal_error_from!(rusqlite::Error);
internal_error_from!(serde_json::Error);
internal_error_from!(axum::extract::multipart::MultipartError);
