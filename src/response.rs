use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

#[derive(Debug, thiserror::Error)]
pub enum ResponseError {
    #[error("{1}")]
    BadRequest(#[source] anyhow::Error, String),

    #[error("Something went wrong.")]
    ServerError(#[from] anyhow::Error),
}

#[derive(Debug, serde::Serialize)]
struct MessageBody {
    pub message: String,
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        match self {
            ResponseError::BadRequest(e, message) => {
                tracing::error!("{:?}", e);
                (StatusCode::BAD_REQUEST, Json(MessageBody { message })).into_response()
            }
            ResponseError::ServerError(e) => {
                tracing::error!("{:?}", e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
