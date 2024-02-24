use crate::server::api_errors::ApiError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::borrow::Cow;

#[allow(dead_code)]
pub type HttpResult = Result<Response, ApiError>;

#[derive(Debug)]
#[repr(transparent)]
pub struct OptionalResult<T>(pub Option<T>);

impl<T: IntoResponse> IntoResponse for OptionalResult<T> {
    fn into_response(self) -> Response {
        self.0
            .map(|result| (StatusCode::OK, result).into_response())
            .unwrap_or_else(|| StatusCode::NOT_FOUND.into_response())
    }
}

impl<T: IntoResponse> From<Option<T>> for OptionalResult<T> {
    fn from(result: Option<T>) -> Self {
        Self(result)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        error!(error=?self, "DMR: responding with ERROR!!");
        let error: anyhow::Error = self.into();
        HttpError::from(error).into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorReport {
    pub error: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<String>,
}

impl From<anyhow::Error> for ErrorReport {
    fn from(error: anyhow::Error) -> Self {
        Self {
            error: error.to_string(),
            error_code: None,
            backtrace: Some(error.backtrace().to_string()),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum HttpError {
    BadRequest { error: ErrorReport },
    NotFound { message: Cow<'static, str> },
    Internal { error: ErrorReport },
}

impl From<anyhow::Error> for HttpError {
    fn from(error: anyhow::Error) -> Self {
        error!("HTTP handler error: {error:?}");
        match error.downcast_ref::<ApiError>() {
            Some(ApiError::Path(_)) => Self::BadRequest { error: error.into() },
            Some(
                ApiError::Registrar(_)
                | ApiError::UpdateWeather(_)
                | ApiError::Noaa(_)
                | ApiError::Json(_)
                | ApiError::HttpEngine(_)
                | ApiError::IO(_)
                | ApiError::Sql(_)
                | ApiError::Database { .. }
                | ApiError::TaskJoin(_),
            ) => Self::Internal { error: error.into() },

            Some(ApiError::Bootstrap(_)) => Self::Internal { error: error.into() },
            None => Self::Internal { error: error.into() },
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound { message } => (StatusCode::NOT_FOUND, Json(message)).into_response(),
            Self::BadRequest { error } => (StatusCode::BAD_REQUEST, Json(error)).into_response(),
            Self::Internal { error } => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}
