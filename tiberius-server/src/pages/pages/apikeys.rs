use crate::pages::common::frontmatter::{csrf_input_tag, form_method, form_submit_button};
use axum::{extract::State, http::HeaderMap, Extension, Router};
use axum_extra::routing::{RouterExt, TypedPath};
use maud::html;
use serde::Deserialize;
use tiberius_core::{
    acl::*,
    app::PageTitle,
    error::{TiberiusError, TiberiusResult},
    request_helper::{FormMethod, HtmlResponse, JsonResponse, TiberiusResponse},
    session::{Authenticated, SessionMode},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{ApiKey, Image, User};
use uuid::Uuid;

pub fn api_key_pages(r: Router<TiberiusState>) -> Router<TiberiusState> {
    r.typed_get(manage_keys_page)
        .typed_post(create_api_key)
        .typed_delete(delete_api_key)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v3/manage/keys")]
pub struct PathManageAPIKeys {}

#[instrument(skip(state, rstate))]
pub async fn manage_keys_page(
    _: PathManageAPIKeys,
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Authenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let view_all_api_keys: bool =
        verify_acl(&state, &rstate, ACLObject::APIKey, ACLActionAPIKey::ViewAll).await?;
    let edit_api_key: bool = verify_acl(
        &state,
        &rstate,
        ACLObject::APIKey,
        ACLActionAPIKey::CreateDelete,
    )
    .await?;
    let admin_api_key: bool =
        verify_acl(&state, &rstate, ACLObject::APIKey, ACLActionAPIKey::Admin).await?;
    let mut client = state.get_db_client();
    let user = rstate.session().get_user(&mut client).await?;
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
                    td { (api_key.id().to_string()) }
                    td { (api_key.secret()) }
                    td {
                        form method="POST" action=(PathDeleteApiKey{uuid: *api_key.id()}.to_uri().to_string()) {
                            (csrf_input_tag(&rstate).await);
                            (form_method(FormMethod::Delete));
                            (form_submit_button("Delete Key"));
                        }
                    }
                }
            }
        }
        @if edit_api_key {
            form method="POST" action=(PathApiCreateAPIKey{}.to_uri().to_string()) {
                (csrf_input_tag(&rstate).await);
                input type="submit" value="Create new Key";
            }
        }
    };
    let app = crate::pages::common::frontmatter::app(
        &state,
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

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v3/manage/keys/create")]
pub struct PathApiCreateAPIKey {}

#[instrument(skip(state, rstate))]
pub async fn create_api_key(
    _: PathApiCreateAPIKey,
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Authenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let edit_api_key: bool = verify_acl(
        &state,
        &rstate,
        ACLObject::APIKey,
        ACLActionAPIKey::CreateDelete,
    )
    .await?;
    let admin_api_key: bool =
        verify_acl(&state, &rstate, ACLObject::APIKey, ACLActionAPIKey::Admin).await?;
    let mut client = state.get_db_client();
    let user = rstate.user(&state).await?;
    let user = match user {
        None => return Err(TiberiusError::AccessDenied),
        Some(v) => v,
    };
    let new_key = ApiKey::new(&user)?;

    let id = new_key.insert(&mut client).await?;
    let key = ApiKey::get_id(&mut client, id)
        .await?
        .expect("we just inserted, cannot fail");

    Ok(TiberiusResponse::Json(JsonResponse {
        content: serde_json::to_value(key)?,
        headers: HeaderMap::new(),
    }))
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v3/manage/keys/:uuid/delete")]
pub struct PathDeleteApiKey {
    uuid: Uuid,
}

#[instrument(skip(state, rstate))]
pub async fn delete_api_key(
    PathDeleteApiKey { uuid }: PathDeleteApiKey,
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Authenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let edit_api_key: bool = verify_acl(
        &state,
        &rstate,
        ACLObject::APIKey,
        ACLActionAPIKey::CreateDelete,
    )
    .await?;
    let admin_api_key: bool =
        verify_acl(&state, &rstate, ACLObject::APIKey, ACLActionAPIKey::Admin).await?;
    let mut client = state.get_db_client();
    let api_key = ApiKey::get_id(&mut client, uuid).await?;

    let api_key = match api_key {
        None => return Err(TiberiusError::AccessDenied),
        Some(v) => v,
    };

    if Some(api_key.user_id()) != rstate.user(&state).await?.as_ref().map(|x| x.id())
        && !admin_api_key
    {
        return Err(TiberiusError::AccessDenied);
    }

    let ok = api_key.clone().delete(&mut client).await?;

    Ok(TiberiusResponse::Json(JsonResponse {
        content: serde_json::to_value(ok)?,
        headers: HeaderMap::new(),
    }))
}
