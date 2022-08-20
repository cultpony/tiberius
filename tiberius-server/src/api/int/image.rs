use std::borrow::Cow;

use axum_extra::routing::TypedPath;
use serde::Deserialize;

#[derive(TypedPath, Deserialize)]
#[typed_path("/image/favorite")]
pub struct PathFavoriteImage {}

#[instrument(level = "trace")]
pub async fn favorite(_: PathFavoriteImage) -> String {
    todo!();
}
