use axum::Json;
use axum_extra::routing::TypedPath;
use serde::Deserialize;
use tiberius_core::error::TiberiusResult;

#[derive(TypedPath, Deserialize)]
#[typed_path("/oembed")]
pub struct PathOembed {}

#[instrument]
pub async fn fetch(_: PathOembed) -> TiberiusResult<Json<()>> {
    todo!()
}
