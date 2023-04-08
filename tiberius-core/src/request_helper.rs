use std::{borrow::Cow, convert::TryInto};

use axum::{headers::ContentType, middleware::Next};
use axum_extra::routing::TypedPath;
use either::Either;
use serde::{de::DeserializeOwned, Deserialize};
use sqlx::{pool::PoolConnection, Pool, Postgres};
use tiberius_dependencies::{
    axum,
    axum::{
        headers::HeaderMapExt,
        response::{IntoResponse, Redirect},
    },
    axum_csrf::{CsrfToken, self},
    axum_flash::Flash,
};
use tiberius_models::{ApiKey, Client, DirectSafeSerialize, Image, SafeSerialize};

use axum::http::{HeaderMap, Request};

use crate::{
    acl::{verify_acl, ACLActionSite, ACLObject},
    app::DBPool,
    config::Configuration,
    error::{TiberiusError, TiberiusResult},
    http_client,
    session::SessionMode,
    state::TiberiusRequestState,
};

pub type DbRef = PoolConnection<Postgres>;

#[derive(serde::Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum FormMethod {
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "update")]
    Update,
}

impl ToString for FormMethod {
    fn to_string(&self) -> String {
        use FormMethod::*;
        match self {
            Delete => "delete",
            Create => "create",
            Update => "update",
        }
        .to_string()
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct ApiFormData<T: std::fmt::Debug> {
    #[serde(rename = "_csrf_token")]
    csrf_token: String,
    #[serde(rename = "_method")]
    method: Option<FormMethod>,
    #[serde(flatten, bound(deserialize = "T: serde::Deserialize<'de>"))]
    pub data: T,
}

#[derive(serde::Deserialize, Debug)]
pub struct ApiFormDataEmpty {
    #[serde(rename = "_csrf_token")]
    csrf_token: String,
    #[serde(rename = "_method")]
    method: Option<FormMethod>,
}

impl ApiFormDataEmpty {
    pub fn into_afd(&self) -> ApiFormData<()> {
        ApiFormData {
            csrf_token: self.csrf_token.clone(),
            method: self.method.clone(),
            data: (),
        }
    }
}

impl<T: std::fmt::Debug> ApiFormData<T> {
    pub fn verify_csrf<R: SessionMode>(
        &self,
        method: Option<FormMethod>,
        rstate: &TiberiusRequestState<R>,
    ) -> bool {
        // verify method expected == method gotten
        if method != self.method {
            false
        } else {
            rstate.csrf_token().verify(&self.csrf_token).is_ok()
        }
    }
    pub fn method(&self) -> Option<FormMethod> {
        self.method
    }
}

pub struct SqlxMiddleware {
    pool: Pool<Postgres>,
}

pub struct ConnectionWrapper {
    pool: Pool<Postgres>,
}

impl SqlxMiddleware {
    pub async fn new(db_conn: Pool<Postgres>) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self { pool: db_conn })
    }
}

pub enum TiberiusResponse<T: IntoResponse> {
    Html(HtmlResponse),
    Json(JsonResponse),
    SafeJson(SafeJsonResponse),
    File(FileResponse),
    Redirect(Redirect),
    Custom(CustomResponse<T>),
    Error(TiberiusError),
    Other(T),
}

impl<T> IntoResponse for TiberiusResponse<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> axum::response::Response {
        match self {
            TiberiusResponse::Html(h) => h.into_response(),
            TiberiusResponse::Json(j) => j.into_response(),
            TiberiusResponse::SafeJson(j) => j.into_response(),
            TiberiusResponse::File(f) => f.into_response(),
            TiberiusResponse::Redirect(r) => r.into_response(),
            TiberiusResponse::Custom(v) => v.into_response(),
            TiberiusResponse::Error(e) => e.into_response(),
            TiberiusResponse::Other(v) => v.into_response(),
        }
    }
}

impl<T> TiberiusResponse<T> where T: IntoResponse {
    pub fn with_flash(self, flash: Flash) -> TiberiusResponse<(Flash, TiberiusResponse<T>)> {
        TiberiusResponse::Other((flash, self))
    }
}

pub struct HtmlResponse {
    pub content: String,
}

impl IntoResponse for HtmlResponse {
    fn into_response(self) -> axum::response::Response {
        let mut hm = HeaderMap::new();
        hm.typed_insert(ContentType::html());
        (hm, self.content).into_response()
    }
}

impl From<String> for HtmlResponse {
    fn from(s: String) -> Self {
        Self { content: s }
    }
}

impl From<maud::PreEscaped<String>> for HtmlResponse {
    fn from(s: maud::PreEscaped<String>) -> Self {
        Self { content: s.0 }
    }
}

pub struct RedirectResponse {
    pub redirect: Redirect,
}

impl IntoResponse for RedirectResponse {
    fn into_response(self) -> axum::response::Response {
        self.redirect.into_response()
    }
}

impl RedirectResponse {
    pub fn new<T>(uri: axum::http::Uri) -> Self {
        Self {
            redirect: Redirect::to(uri.to_string().as_str()),
        }
    }
}

pub struct JsonResponse {
    pub content: serde_json::Value,
    pub headers: HeaderMap,
}

impl IntoResponse for JsonResponse {
    fn into_response(self) -> axum::response::Response {
        (self.headers, serde_json::to_string(&self.content).unwrap()).into_response()
    }
}

impl JsonResponse {
    pub fn safe_serialize<T: SafeSerialize>(v: T, headers: HeaderMap) -> TiberiusResult<Self> {
        Ok(JsonResponse {
            content: serde_json::to_value(v.into_safe())?,
            headers,
        })
    }
    pub fn direct_safe_serialize<T: DirectSafeSerialize>(
        v: &T,
        headers: HeaderMap,
    ) -> TiberiusResult<Self> {
        Ok(JsonResponse {
            content: serde_json::to_value(&v)?,
            headers,
        })
    }
}

pub struct SafeJsonResponse {
    pub content: serde_json::Value,
}

impl SafeJsonResponse {
    pub fn safe_serialize<T: SafeSerialize>(v: T) -> TiberiusResult<Self> {
        Ok(SafeJsonResponse {
            content: serde_json::to_value(v.into_safe())?,
        })
    }
    pub fn direct_safe_serialize<T: DirectSafeSerialize>(v: &T) -> TiberiusResult<Self> {
        Ok(SafeJsonResponse {
            content: serde_json::to_value(&v)?,
        })
    }
}

impl IntoResponse for SafeJsonResponse {
    fn into_response(self) -> axum::response::Response {
        serde_json::to_string(&self.content)
            .unwrap()
            .into_response()
    }
}

pub struct FileResponse {
    pub content: Cow<'static, [u8]>,
    pub headers: HeaderMap,
}

impl IntoResponse for FileResponse {
    fn into_response(self) -> axum::response::Response {
        (self.headers, self.content).into_response()
    }
}

pub struct CustomResponse<T: IntoResponse> {
    pub content: T,
    pub headers: HeaderMap,
}

impl<T> IntoResponse for CustomResponse<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> axum::response::Response {
        (self.headers, self.content).into_response()
    }
}
