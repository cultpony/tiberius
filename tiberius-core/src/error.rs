use rocket::http::ContentType;
use rocket::{http::Status, response::Responder, Response};
use std::io::Cursor;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TiberiusError {
    #[error("Database Error: {0}")]
    Database(#[from] tiberius_models::PhilomenaModelError),
    #[error("SQLx Error: {0}")]
    SQLx(#[from] sqlx::Error),
    #[error("SQL Migration Error: {0}")]
    SQLMigration(#[from] sqlx::migrate::MigrateError),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Ring: Unspecified: {0}")]
    RingUnspec(#[from] ring::error::Unspecified),
    #[error("Ring: Key Rejected: {0}")]
    RingKR(#[from] ring::error::KeyRejected),
    #[error("Envy Error: {0}")]
    Envy(#[from] envy::Error),
    #[error("Serde: JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Serde: CBOR: {0}")]
    SerdeCbor(#[from] serde_cbor::Error),
    #[error("Paseto: {0}")]
    Paseto(String),
    #[error("Other Error: {0:?}")]
    Other(String),
    //#[error("Infallible Error")]
    //Infallible(#[from] Infallible),
    #[error("ParseInt Error: {0}")]
    ParseIntError(#[from] std::string::ParseError),
    #[error("UUID Error: {0}")]
    ParseUuidError(#[from] uuid::Error),
    #[error("Route {0:?} not implemented")]
    RouteNotFound(String),
    #[error("The page located under {0:?} could not be found, have you tried looking in Celestia's secret stash?")]
    PageNotFound(String),
    #[error("Something went wrong trying to redirect you with a flash")]
    FlashRedirect(rocket::response::Flash<rocket::response::Redirect>),
    #[error("Something went wrong trying to redirect you")]
    Redirect(rocket::response::Redirect),
    #[error("Could not join thread: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Could not parse URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("Search Index Error: {0}")]
    TantivyTopLevel(#[from] tiberius_models::TantivyError),
    #[error("Web Engine Error: {0}")]
    Rocket(#[from] rocket::Error),
    #[error("BCrypt Error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("Error in Time: {0}")]
    DoctorWho(#[from] std::time::SystemTimeError),
    #[error("Couldn't strip path prefix: {0}")]
    StripPathPrefix(#[from] std::path::StripPrefixError),
    #[error("Could not process image: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Database Error in ACL Engine: {0}")]
    CasbinSqlError(#[from] sqlx_adapter::Error),
    #[error("Error in ACL Engine: {0}")]
    CasbinError(#[from] sqlx_adapter::casbin::Error),
    #[error("Access has been denied")]
    AccessDenied,
    #[error("Configuration Variable Unset: {0}")]
    ConfigurationUnset(String),
    #[error("OpenSSL Failure: {0}")]
    OpenSSL(#[from] openssl::error::Error),
    #[error("OpenSSL Multiple Failures: {0:?}")]
    OpenSSLComplex(#[from] openssl::error::ErrorStack),
    #[error("Invalid Philomena Cookie")]
    InvalidPhilomenaCookie,
    #[error("Could not decode base64 string: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Erlang Error: {0}")]
    ErlangTermDecode(String),
    #[error("{0} {0} not found")]
    ObjectNotFound(String, String),
}

pub type TiberiusResult<T> = std::result::Result<T, TiberiusError>;

impl<'r> Responder<'r, 'static> for TiberiusError {
    fn respond_to(self, r: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        match self {
            Self::FlashRedirect(v) => return v.respond_to(r),
            Self::Redirect(v) => return v.respond_to(r),
            _ => (),
        }
        let c = maud::html! {
            "Internal Error"
            br;
            b { (format!("{}", self.to_string())) };
        };
        let c: String = c.into_string();
        Ok(Response::build()
            .status(Status::InternalServerError)
            .header(ContentType::HTML)
            .sized_body(c.bytes().len(), Cursor::new(c))
            .finalize())
    }
}
