use axum_extra::routing::TypedPath;
use serde::Deserialize;

#[derive(TypedPath, Deserialize)]
#[typed_path("/user/:username")]
pub struct PathUserProfile {
    pub username: String,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/user/id/:user_id")]
pub struct PathUserProfileId {
    pub user_id: i64,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/avatar/*path")]
pub struct PathUserAvatar {
    pub path: String,
}
