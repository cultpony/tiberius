use std::borrow::Cow;
use std::convert::TryInto;

use either::Either;
use rocket::http::uri::Reference;
use rocket::{form::FromForm, http::ContentType, Request, State};
use sqlx::{pool::PoolConnection, Pool, Postgres};
use tiberius_models::{ApiKey, Client, Image, DirectSafeSerialize, SafeSerialize};

use crate::state::Flash;
use crate::{
    app::DBPool,
    config::Configuration,
    error::{TiberiusError, TiberiusResult},
    http_client,
};

pub type DbRef = PoolConnection<Postgres>;

#[derive(serde::Deserialize, Copy, Clone, PartialEq, Eq, rocket::form::FromFormField, Debug)]
pub enum FormMethod {
    #[serde(rename = "delete")]
    #[field(value = "delete")]
    Delete,
    #[serde(rename = "create")]
    #[field(value = "create")]
    Create,
    #[serde(rename = "update")]
    #[field(value = "update")]
    Update,
}

impl ToString for FormMethod {
    fn to_string(&self) -> String {
        use FormMethod::*;
        match self {
            Delete => "delete",
            Create => "create",
            Update => "update",
        }.to_string()
    }
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
#[serde(transparent)]
pub struct CSRFToken(String);

#[rocket::async_trait]
impl<'r> FromForm<'r> for CSRFToken {
    type Context = String;

    fn init(opts: rocket::form::Options) -> Self::Context {
        "".to_string()
    }

    fn push_value(ctxt: &mut Self::Context, field: rocket::form::ValueField<'r>) {
        if field.name == "_csrf_token" {
            *ctxt = field.value.to_string()
        }
    }

    async fn push_data(ctxt: &mut Self::Context, field: rocket::form::DataField<'r, '_>) {
        // noop
    }

    fn finalize(ctxt: Self::Context) -> rocket::form::Result<'r, Self> {
        Ok(CSRFToken(ctxt))
    }
}

impl Into<String> for CSRFToken {
    fn into(self) -> String {
        self.0
    }
}

#[derive(serde::Deserialize, rocket::form::FromForm)]
pub struct ApiFormData<T> {
    #[serde(rename = "_csrf_token")]
    #[field(name = "_csrf_token")]
    csrf_token: CSRFToken,
    #[serde(rename = "_method")]
    #[field(name = "_method")]
    method: Option<FormMethod>,
    #[serde(flatten, bound(deserialize = "T: serde::Deserialize<'de>"))]
    pub data: T,
}

#[derive(serde::Deserialize, rocket::form::FromForm)]
pub struct ApiFormDataEmpty {
    #[serde(rename = "_csrf_token")]
    #[field(name = "_csrf_token")]
    csrf_token: CSRFToken,
    #[serde(rename = "_method")]
    #[field(name = "_method")]
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

impl<T> ApiFormData<T> {
    pub fn verify_csrf(&self, method: Option<FormMethod>) -> bool {
        // verify method expected == method gotten
        if method != self.method {
            return false;
        }
        //TODO: verify CSRF valid!
        true
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

#[derive(rocket::Responder)]
pub enum TiberiusResponse<T> {
    Html(HtmlResponse),
    Json(JsonResponse),
    JsonNoHeader(HlJsonResponse),
    SafeJson(SafeJsonResponse),
    File(FileResponse),
    Redirect(RedirectResponse),
    NoFlashRedirect(NonFlashRedirectResponse),
    Custom(CustomResponse<T>),
    Error(TiberiusError),
}

#[derive(rocket::Responder)]
#[response(status = 200, content_type = "html")]
pub struct HtmlResponse {
    pub content: String,
}

#[derive(rocket::Responder)]
#[response()]
pub struct RedirectResponse {
    pub redirect: rocket::response::Flash<rocket::response::Redirect>,
}

impl RedirectResponse {
    pub fn new<T, S: TryInto<Reference<'static>>>(
        uri: S,
        flash: Option<Flash>,
    ) -> TiberiusResponse<T> {
        match flash {
            Some(flash) => TiberiusResponse::Redirect(Self {
                redirect: flash.into_resp(rocket::response::Redirect::to(uri)),
            }),
            None => TiberiusResponse::NoFlashRedirect(NonFlashRedirectResponse {
                redirect: rocket::response::Redirect::to(uri),
            }),
        }
    }
}

#[derive(rocket::Responder)]
#[response()]
pub struct NonFlashRedirectResponse {
    pub redirect: rocket::response::Redirect,
}

#[derive(rocket::Responder)]
#[response(status = 200, content_type = "json")]
pub struct JsonResponse {
    pub content: serde_json::Value,
    pub headers: rocket::http::Header<'static>,
}

impl JsonResponse {
    pub fn safe_serialize<T: SafeSerialize>(v: &T, headers: rocket::http::Header<'static>) -> TiberiusResult<Self> {
        Ok(JsonResponse {
            content: serde_json::to_value(v.into_safe())?,
            headers,
        })
    }
    pub fn direct_safe_serialize<T: DirectSafeSerialize>(v: &T, headers: rocket::http::Header<'static>) -> TiberiusResult<Self> {
        Ok(JsonResponse {
            content: serde_json::to_value(&v)?,
            headers,
        })
    }
}

#[derive(rocket::Responder)]
#[response(status = 200, content_type = "json")]
pub struct HlJsonResponse {
    pub content: serde_json::Value,
}

impl HlJsonResponse {
    pub fn safe_serialize<T: SafeSerialize>(v: &T) -> TiberiusResult<Self> {
        Ok(HlJsonResponse {
            content: serde_json::to_value(v.into_safe())?,
        })
    }
    pub fn direct_safe_serialize<T: DirectSafeSerialize>(v: &T) -> TiberiusResult<Self> {
        Ok(HlJsonResponse {
            content: serde_json::to_value(&v)?,
        })
    }
}

#[derive(rocket::Responder)]
#[response(status = 200, content_type = "json")]
pub struct SafeJsonResponse {
    pub content: serde_json::Value,
}

impl SafeJsonResponse {
    pub fn safe_serialize<T: SafeSerialize>(v: &T) -> TiberiusResult<Self> {
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

#[derive(rocket::Responder)]
#[response(status = 200)]
pub struct FileResponse {
    pub content: Cow<'static, [u8]>,
    pub content_type: ContentType,
}

#[derive(rocket::Responder)]
#[response(status = 200)]
pub struct CustomResponse<T> {
    #[response(bound = "T: rocket::response::Responder")]
    pub content: T,
    pub content_type: ContentType,
}
