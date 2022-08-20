use axum_extra::routing::TypedPath;
use maud::Markup;
use serde::Deserialize;
use tiberius_core::{error::TiberiusResult, request_helper::HtmlResponse};

pub mod staff_page;

#[derive(TypedPath, Deserialize)]
#[typed_path("/pages/:page")]
pub struct PathBlogPage {
    pub page: String,
}

pub async fn show(PathBlogPage { page }: PathBlogPage) -> TiberiusResult<HtmlResponse> {
    todo!()
}
