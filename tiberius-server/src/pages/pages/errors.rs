use rocket::Request;
use tiberius_core::error::TiberiusError;
use tiberius_core::request_helper::{HtmlResponse, TiberiusResponse};

use crate::pages::error_page;

#[catch(500)]
pub async fn server_error() -> TiberiusResponse<()> {
    let content = error_page(&TiberiusError::Other(format!(
        "Sorry for that, we encountered an issue with your request."
    )))
    .await
    .into_string();
    TiberiusResponse::Html(HtmlResponse { content })
}
