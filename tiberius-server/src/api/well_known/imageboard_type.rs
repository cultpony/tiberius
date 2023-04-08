use axum_extra::routing::TypedPath;
use serde::Deserialize;

use crate::package_full;

#[derive(TypedPath, Deserialize)]
#[typed_path("/.well-known/imageboard-type")]
pub struct PathImageBoardApiFlavor {}

#[instrument]
pub async fn imageboardtype(_: PathImageBoardApiFlavor) -> String {
    format!(
        "{},min-api:{},max-api:{},api-flavor:{},flavor-philomena-int:{},flavor-philomena:{}",
        package_full(),
        1,
        1,
        "tiberius",
        "!1",
        "!1"
    )
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/.well-known/imageboard-api/flavor-tiberius")]
pub struct PathImageBoardTiberiusApiFlavor {}

#[instrument]
pub async fn imageboardapiflavor(_: PathImageBoardTiberiusApiFlavor) -> String {
    "/api/v1".to_string()
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/.well-known/imageboard-api/flavor-philomena-int")]
pub struct PathImageBoardPhilomenaIntApiFlavor {}

#[instrument]
pub async fn imageboardapiflavor_philomena_int(_: PathImageBoardPhilomenaIntApiFlavor) -> String {
    "!".to_string()
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/.well-known/imageboard-api/flavor-philomena")]
pub struct PathImageBoardPhilomenaApiFlavor {}

#[instrument]
pub async fn imageboardapiflavor_philomena_v1(_: PathImageBoardPhilomenaApiFlavor) -> String {
    "/api/philomena/v1".to_string()
}
