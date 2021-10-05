use rocket::form::Form;
use rocket::State;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::request_helper::JsonResponse;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Image, User};

use crate::pages::common::{verify_acl, ACLActionImage, ACLObject, ACLSubject};

#[derive(FromForm, serde::Serialize)]
pub struct ChangeUploader {
    new_uploader: String,
    old_uploader: String,
}

#[get("/api/v3/images/<image>/change_uploader", data = "<change_uploader>")]
pub async fn change_image_uploader(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_>,
    image: u64,
    change_uploader: Form<ChangeUploader>,
) -> TiberiusResult<JsonResponse> {
    let mut client = state.get_db_client().await?;
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
    let new_uploader = User::get_by_name(&mut client, change_uploader.new_uploader.clone()).await?;
    let new_uploader = match new_uploader {
        Some(v) => v,
        None => {
            return Err(TiberiusError::ObjectNotFound(
                "User".to_string(),
                change_uploader.new_uploader.clone(),
            ))
        }
    };
    let old_uploader = User::get_by_name(&mut client, change_uploader.old_uploader.clone()).await?;
    let old_uploader = match old_uploader {
        Some(v) => v,
        None => {
            return Err(TiberiusError::ObjectNotFound(
                "User".to_string(),
                change_uploader.old_uploader.clone(),
            ))
        }
    };
    let image = Image::get_id(&mut client, image as i64).await?;
    let mut image = match image {
        Some(v) => v,
        None => return Err(TiberiusError::AccessDenied),
    };
    if image.user_id != Some(old_uploader.id) {
        return Err(TiberiusError::AccessDenied);
    }
    image.user_id = Some(new_uploader.id);
    todo!("issue reindex to philomena if necessary");
    todo!("save to database");
    todo!("return OK json");
}
