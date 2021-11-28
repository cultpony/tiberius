use maud::html;
use rocket::form::Form;
use rocket::State;
use tiberius_core::app::PageTitle;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::request_helper::{FormMethod, HlJsonResponse, HtmlResponse, JsonResponse, TiberiusResponse};
use tiberius_core::session::{Authenticated, SessionMode};
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{ApiKey, Image, User};
use uuid::Uuid;
use crate::pages::common::frontmatter::{form_submit_button, form_method, csrf_input_tag};

use crate::pages::common::{verify_acl, ACLActionAPIKey, ACLActionImage, ACLObject, ACLSubject};

#[get("/v3/manage/keys")]
pub async fn manage_keys_page(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let view_all_api_keys: bool =
        verify_acl(state, &rstate, ACLObject::APIKey, ACLActionAPIKey::ViewAll).await?;
    let edit_api_key: bool = verify_acl(
        state,
        &rstate,
        ACLObject::APIKey,
        ACLActionAPIKey::CreateDelete,
    )
    .await?;
    let admin_api_key: bool =
        verify_acl(state, &rstate, ACLObject::APIKey, ACLActionAPIKey::Admin).await?;
    let mut client = state.get_db_client().await?;
    let user = rstate.session.read().await.get_user(&mut client).await?;
    let user = match user {
        Some(u) => u,
        None => return Err(TiberiusError::AccessDenied),
    };
    let keys: Vec<ApiKey> = if !view_all_api_keys {
        ApiKey::get_all_of_user(&mut client, &user, None, None).await?
    } else {
        ApiKey::get_all(&mut client, None, None).await?
    };
    let body = html! {
        table {
            tr {
                th { "User" };
                th { "Key ID" };
                th { "Key Secret" };
                th { "Actions" };
            }
            @for api_key in keys {
                tr {
                    td { (api_key.user(&mut client).await?.expect("key has no user").displayname()) }
                    td { (api_key.id()) }
                    td { (api_key.secret()) }
                    td {
                        form method="POST" action=(uri!(delete_api_key(api_key.id()))) {
                            (csrf_input_tag(&rstate).await);
                            (form_method(FormMethod::Delete));
                            (form_submit_button("Delete Key"));
                        }
                    }
                }
            }
        }
        @if edit_api_key {
            form method="POST" action=(uri!(create_api_key)) {
                input type="submit" value="Create new Key";
            }
        }
    };
    let app = crate::pages::common::frontmatter::app(
        state,
        &rstate,
        Some(PageTitle::from("API - Manage API Keys")),
        &mut client,
        body,
        None,
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}

#[post("/v3/manage/keys/create")]
pub async fn create_api_key(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
) -> TiberiusResult<()> {
    let edit_api_key: bool = verify_acl(
        state,
        &rstate,
        ACLObject::APIKey,
        ACLActionAPIKey::CreateDelete,
    )
    .await?;
    let admin_api_key: bool =
        verify_acl(state, &rstate, ACLObject::APIKey, ACLActionAPIKey::Admin).await?;
    let mut client = state.get_db_client().await?;
    let user = rstate.user(state).await?;
    let user = match user {
        None => return Err(TiberiusError::AccessDenied),
        Some(v) => v,
    };
    let new_key = ApiKey::new(&user)?;

    let id = new_key.insert(&mut client).await?;
    let key = ApiKey::get_id(&mut client, id).await?;

    todo!("Return JSON Response");
}

#[post("/v3/manage/keys/<uuid>/delete")]
pub async fn delete_api_key(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
    uuid: Uuid,
) -> TiberiusResult<()> {
    let edit_api_key: bool = verify_acl(
        state,
        &rstate,
        ACLObject::APIKey,
        ACLActionAPIKey::CreateDelete,
    )
    .await?;
    let admin_api_key: bool =
        verify_acl(state, &rstate, ACLObject::APIKey, ACLActionAPIKey::Admin).await?;
    let mut client = state.get_db_client().await?;
    let api_key = ApiKey::get_id(&mut client, uuid).await?;

    let api_key = match api_key {
        None => return Err(TiberiusError::AccessDenied),
        Some(v) => v,
    };

    if Some(api_key.user_id()) != rstate.user(&state).await?.map(|x| x.id()) {
        if !admin_api_key {
            return Err(TiberiusError::AccessDenied);
        }
    }

    let ok = api_key.clone().delete(&mut client).await?;

    Ok(TiberiusResponse::JsonNoHeader(HlJsonResponse{
        content: serde_json::to_string(ok),
    }))
}
