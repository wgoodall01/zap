use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API error wrapper that preserves the full error cause chain
/// while mapping to HTTP status codes
#[derive(Debug)]
pub struct ApiError {
    pub inner: anyhow::Error,
    pub code: StatusCode,
}

impl ApiError {
    /// Create a new API error with a specific status code
    pub fn new(inner: anyhow::Error, code: StatusCode) -> Self {
        Self { inner, code }
    }

    /// Create a 400 Bad Request error
    pub fn bad_request(inner: anyhow::Error) -> Self {
        Self::new(inner, StatusCode::BAD_REQUEST)
    }

    /// Create a 401 Unauthorized error
    pub fn unauthorized(inner: anyhow::Error) -> Self {
        Self::new(inner, StatusCode::UNAUTHORIZED)
    }

    /// Create a 404 Not Found error
    pub fn not_found(inner: anyhow::Error) -> Self {
        Self::new(inner, StatusCode::NOT_FOUND)
    }

    /// Create a 500 Internal Server Error
    pub fn internal_server_error(inner: anyhow::Error) -> Self {
        Self::new(inner, StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Error response body
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Vec<String>>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Collect the full error chain for the response
        let mut chain = Vec::new();

        // Start with the root error message
        chain.push(self.inner.to_string());

        // Walk the error chain
        let mut current_error: &dyn std::error::Error = self.inner.as_ref();
        while let Some(source) = current_error.source() {
            chain.push(source.to_string());
            current_error = source;
        }

        let message = chain.first().cloned().unwrap_or_else(|| "Unknown error".to_string());
        let details = if chain.len() > 1 {
            Some(chain[1..].to_vec())
        } else {
            None
        };

        // Log the error with full chain using Display formatting
        // The error chain is built in the same way as the response
        if let Some(details) = &details {
            tracing::error!(
                status_code = %self.code,
                error = %message,
                causes = ?details,
                "request failed with error"
            );
        } else {
            tracing::error!(
                status_code = %self.code,
                error = %message,
                "request failed with error"
            );
        }

        let body = ErrorResponse {
            error: message,
            details,
        };

        (self.code, Json(body)).into_response()
    }
}

/// Automatically convert anyhow::Error to ApiError with 500 status
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal_server_error(err)
    }
}

/// Convert sqlx errors to ApiError
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::internal_server_error(anyhow::Error::from(err))
    }
}

/// Convert reqwest errors to ApiError
impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() || err.is_connect() {
            Self::new(
                anyhow::Error::from(err),
                StatusCode::GATEWAY_TIMEOUT,
            )
        } else if err.is_status() {
            Self::new(
                anyhow::Error::from(err),
                StatusCode::BAD_GATEWAY,
            )
        } else {
            Self::internal_server_error(anyhow::Error::from(err))
        }
    }
}

/// Convenience type alias for API results
pub type ApiResult<T> = Result<T, ApiError>;
