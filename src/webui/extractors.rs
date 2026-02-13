use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    Json,
};
use garde::Validate;
use serde::de::DeserializeOwned;

use super::error::ApiError;

/// Axum extractor that deserializes JSON and validates via `garde`.
///
/// Usage: replace `Json<T>` with `ValidJson<T>` where `T: garde::Validate`.
pub struct ValidJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate<Context = ()> + 'static,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ApiError::BadRequest(e.body_text()))?;

        value
            .validate()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        Ok(ValidJson(value))
    }
}
