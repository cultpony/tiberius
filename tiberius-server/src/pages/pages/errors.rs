use axum::http::Request;
use tiberius_core::{
    error::TiberiusError,
    request_helper::{HtmlResponse, TiberiusResponse},
};

use crate::pages::error_page;

pub async fn server_error() -> TiberiusResponse<()> {
    let content = error_page(&TiberiusError::Other(format!(
        "Sorry for that, we encountered an issue with your request."
    )))
    .await
    .into_string();
    TiberiusResponse::Html(HtmlResponse { content })
}

pub async fn access_denied() -> String {
    format!("Access Denied")
}
