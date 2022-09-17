use axum::Extension;
use axum_extra::routing::TypedPath;
use serde::Deserialize;
use tiberius_core::{
    error::{TiberiusError, TiberiusResult},
    request_helper::{SafeJsonResponse, TiberiusResponse},
    session::{SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{Image, User};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v3/images/:image")]
pub struct ApiV3ImageGetImageData {
    image: u64,
}

#[instrument(skip(state, rstate))]
pub async fn get_image_data(
    Extension(state): Extension<TiberiusState>,
    Extension(rstate): Extension<TiberiusRequestState<Unauthenticated>>,
    image: u64,
) -> TiberiusResult<SafeJsonResponse> {
    let mut client = state.get_db_client();
    let image = Image::get_id(&mut client, image as i64).await?;
    match image {
        Some(image) => Ok(SafeJsonResponse::safe_serialize(&image)?),
        None => Err(TiberiusError::PageNotFound("image".to_string())),
    }
}
