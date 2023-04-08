use axum::{Extension, Form, extract::State};
use axum_extra::routing::TypedPath;
use maud::html;
use tiberius_core::{
    acl::*,
    app::PageTitle,
    error::{TiberiusError, TiberiusResult},
    request_helper::{HtmlResponse, JsonResponse, SafeJsonResponse, TiberiusResponse},
    session::{Authenticated, SessionMode},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{Client, Image, User};

#[derive(serde::Serialize, Debug)]
pub struct ChangeUploader {
    new_uploader: String,
    old_uploader: String,
}

#[derive(serde::Deserialize, TypedPath, Debug)]
#[typed_path("/api/v3/images/:image/change_uploader")]
pub struct PathApiV3ImageChangeUploader {
    image: u64,
}

#[instrument(skip(state, rstate))]
pub async fn change_image_uploader_user(
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Authenticated>,
    PathApiV3ImageChangeUploader { image }: PathApiV3ImageChangeUploader,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client = state.get_db_client();
    let body = html! {
        form action=(PathApiV3ImageChangeUploader{ image }.to_uri().to_string()) method="POST" {
            label for="old_uploader" { "Old Uploader" }
            input type="text" name="old_uploader" id="old_uploader" placeholder="old_uploader" {}
            label for="new_uploader" { "New Uploader" }
            input type="text" name="new_uploader" id="new_uploader" placeholder="new_uploader" {}
            input type="submit" value="Submit" { "Submit" }
        }
    };
    let app = crate::pages::common::frontmatter::app(
        &state,
        &rstate,
        Some(PageTitle::from("API - Change Uploader")),
        &mut client,
        body,
        None,
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}

#[instrument(skip(state, rstate))]
pub async fn change_image_uploader(
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Authenticated>,
    PathApiV3ImageChangeUploader { image }: PathApiV3ImageChangeUploader,
    Form(change_uploader): Form<ChangeUploader>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client = state.get_db_client();
    let verify_acl = verify_acl(
        &state,
        &rstate,
        ACLObject::Image,
        ACLActionImage::ChangeUploader,
    )
    .await?;
    if !verify_acl {
        return Err(TiberiusError::AccessDenied);
    }
    let new_uploader = User::get_by_name(&mut client, &change_uploader.new_uploader).await?;
    let new_uploader = match new_uploader {
        Some(v) => v,
        None => {
            return Err(TiberiusError::ObjectNotFound(
                "User".to_string(),
                change_uploader.new_uploader.clone(),
            ))
        }
    };
    let old_uploader = User::get_by_name(&mut client, &change_uploader.old_uploader).await?;
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
    //TODO: issue reindex to philomena if necessary
    let image = image.save(&mut client).await?;
    Ok(TiberiusResponse::SafeJson(
        SafeJsonResponse::safe_serialize(image)?,
    ))
}

#[cfg(test)]
mod test {
    use crate::api::v3::images::ChangeUploader;
    use tiberius_core::{app::DBPool, config::Configuration, error::TiberiusResult};

    // TODO: make sure this test works again
    /*#[sqlx_database_tester::test(
        pool(variable = "pool", migrations = "../migrations"),
    )]
    async fn test_change_uploader_reject_unauthoriezd() -> TiberiusResult<()> {
        let mut config = Configuration::default();
        unsafe { config.set_alt_dbconn(pool.clone()) };
        let rocket = rocket(pool, &config).await.unwrap();
        let client = Client::tracked(rocket).await.unwrap();

        let resp = client.post("/api/v3/images/0/change_uploader")
            .json(&ChangeUploader{
                new_uploader: "120".to_string(),
                old_uploader: "100".to_string()
            })
            .dispatch().await;
        assert_eq!(resp.status(), Status::NotFound);
        Ok(())
    }*/
}
