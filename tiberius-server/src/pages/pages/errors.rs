use axum::http::Request;
use tiberius_core::{
    error::TiberiusError,
    request_helper::{HtmlResponse, TiberiusResponse},
};

use crate::pages::error_page;

#[tracing::instrument]
pub async fn server_error() -> TiberiusResponse<()> {
    let content = error_page(&TiberiusError::Other(
        "Sorry for that, we encountered an issue with your request.".to_string(),
    ))
    .await
    .into_string();
    TiberiusResponse::Html(HtmlResponse { content })
}

#[tracing::instrument]
pub async fn access_denied() -> String {
    "Access Denied".to_string()
}
