use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: &'static str,
    #[serde(skip)]
    status: StatusCode,
}

impl ApiError {
    pub fn bad_request(error: &'static str) -> Self {
        Self {
            error,
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn not_found(error: &'static str) -> Self {
        Self {
            error,
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn conflict(error: &'static str) -> Self {
        Self {
            error,
            status: StatusCode::CONFLICT,
        }
    }

    pub fn unauthorized(error: &'static str) -> Self {
        Self {
            error,
            status: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn forbidden(error: &'static str) -> Self {
        Self {
            error,
            status: StatusCode::FORBIDDEN,
        }
    }

    pub fn service_unavailable() -> Self {
        Self {
            error: "Service unavailable, please try again later",
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
