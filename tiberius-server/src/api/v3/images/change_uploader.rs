use rocket::form::Form;
use rocket::State;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::request_helper::JsonResponse;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Image, User};

use crate::pages::common::{verify_acl, ACLActionImage, ACLObject, ACLSubject};

#[derive(FromForm, serde::Serialize)]
pub struct ChangeUploader {
    new_uploader: u64,
    old_uploader: u64,
}

#[get("/api/v3/images/change_uploader/<image>", data = "<change_uploader>")]
pub async fn change_image_uploader(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_>,
    image: u64,
    change_uploader: Form<ChangeUploader>,
) -> TiberiusResult<JsonResponse> {
    let mut client = state.get_db_client().await?;
    let image = Image::get_id(&mut client, image as i64).await?;
    let verify_acl = verify_acl(
        state,
        &rstate,
        ACLObject::Image,
        ACLActionImage::ChangeUploader,
    )
    .await?;
    if !verify_acl {
        return Err(TiberiusError::AccessDenied);
    }
    todo!()
}
