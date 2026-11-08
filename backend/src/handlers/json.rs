use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
};
use serde::de::DeserializeOwned;

use crate::errors::ApiError;

/// Like `axum::Json`, but malformed bodies produce our standard JSON error (400)
/// instead of a plain-text rejection. Also requires `Content-Type: application/json`,
/// which blocks simple cross-site form posts.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(v)) => Ok(ApiJson(v)),
            Err(JsonRejection::MissingJsonContentType(_)) => Err(ApiError::Validation(
                "Content-Type must be application/json".into(),
            )),
            Err(_) => Err(ApiError::Validation("Request body is not valid".into())),
        }
    }
}
