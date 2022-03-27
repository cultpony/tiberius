use rocket::State;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::request_helper::{SafeJsonResponse, TiberiusResponse};
use tiberius_core::session::{Unauthenticated, SessionMode};
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Image, User};

#[get("/api/v3/images/<image>")]
pub async fn get_image_data(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Unauthenticated>,
    image: u64,
) -> TiberiusResult<SafeJsonResponse> {
    let mut client = state.get_db_client().await?;
    let image = Image::get_id(&mut client, image as i64).await?;
    match image {
        Some(image) => Ok(SafeJsonResponse::safe_serialize(&image)?),
        None => Err(TiberiusError::PageNotFound("image".to_string())),
    }
}