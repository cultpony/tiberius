use axum::{
    extract::FromRequest,
    headers::{ContentType, Cookie, HeaderMapExt},
    http::{HeaderMap, Request, Response, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use axum_extra::routing::TypedPath;
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use tiberius_core::{
    acl::{verify_acl, ACLActionSite, ACLObject},
    error::{TiberiusError, TiberiusResult},
    session::Unauthenticated,
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_dependencies::axum_database_sessions::Session;

use crate::pages::session::PathSessionsLogin;
use crate::set_scope_tx;

pub mod activity;
pub mod apikeys;
pub mod blog;
pub mod channels;
pub mod errors;
pub mod filters;
pub mod images;
pub mod session;
pub mod tags;
pub mod user;

pub async fn todo_page<S: Into<String>>(name: S) -> TiberiusResult<Markup> {
    let name: String = name.into();
    let err: TiberiusResult<Markup> = Err(TiberiusError::RouteNotFound(name));
    err
}

pub async fn todo_page_fn<B>(req: Request<B>) -> TiberiusResult<Markup> {
    tracing::error!("ROUTE {:?} WAS NOT IMPLEMENTED!", req.uri().path());
    todo_page(req.uri().path().to_string()).await
}

pub async fn error_page(err: &TiberiusError) -> Markup {
    let error = err.to_string();
    html! {
        (maud::DOCTYPE)
        html {
            head {
                style {
                    (PreEscaped(r#"
                    "#))
                }
            }
            body {
                div.error.wrapper {
                     h1.error.title { "An error occured while processing your request" }
                     main {
                         (error)
                     }
                }
            }
        }
    }
}

pub async fn not_found_page(uri: Uri) -> (StatusCode, HeaderMap, String) {
    set_scope_tx!("Not Found");
    let mut hm = HeaderMap::new();
    hm.typed_insert(ContentType::html());
    info!("Undefined route called at {uri}");
    (
        axum::http::StatusCode::NOT_FOUND,
        hm,
        error_page(&TiberiusError::PageNotFound(uri.to_string()))
            .await
            .into_string(),
    )
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/filters")]
pub struct PathFilters {}
