use axum::{headers::HeaderMapExt, http::HeaderMap};
use std::{io::Cursor, str::ParseBoolError};
use thiserror::Error;
use tiberius_dependencies::{
    axum,
    axum::{
        body::BoxBody,
        headers::ContentType,
        response::{IntoResponse, Response},
    },
    http::StatusCode,
    mime::FromStrError,
    totp_rs,
};

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
    //#[error("Envy Error: {0}")]
    //Envy(#[from] envy::Error),
    #[error("Serde: JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),
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
    #[error("Could not join thread: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Could not parse URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("Search Index Error: {0}")]
    TantivyTopLevel(#[from] tiberius_models::TantivyError),
    #[error("BCrypt Error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("Error in Time: {0}")]
    DoctorWho(#[from] std::time::SystemTimeError),
    #[error("Couldn't strip path prefix: {0}")]
    StripPathPrefix(#[from] std::path::StripPrefixError),
    #[error("Could not process image: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Access has been denied")]
    AccessDenied,
    #[error("Configuration Variable Unset: {0}")]
    ConfigurationUnset(String),
    #[cfg(test)]
    #[error("OpenSSL Failure: {0}")]
    OpenSSL(#[from] openssl::error::Error),
    #[cfg(test)]
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
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error("Requested Session from Static Handler")]
    StaticSession,
    #[error("Required Database but found None")]
    RequestMissingDatabase,
    #[error("TOTP Error: {0:?}")]
    TotpUrlError(totp_rs::TotpUrlError),
    #[error("Invalid Log Level {0:?}")]
    InvalidLogLevel(String),
    #[error("Could not read request: {0:?}")]
    MultipartError(#[from] axum::extract::multipart::MultipartError),
    #[error("Could not parse field: {0:?}")]
    ParseBool(#[from] ParseBoolError),
    #[error("Could not parse value from string: {0:?}")]
    FromStr(#[from] FromStrError),
    #[error("Could not persist temporary file: {0:?}")]
    PersistError(#[from] tiberius_dependencies::tempfile::PersistError),
    #[error("General HTTP Format Error: {0:?}")]
    HttpError(#[from] tiberius_dependencies::http::Error),
    #[error("Serde QS Error: {0:?}")]
    SerdeQsError(#[from] tiberius_dependencies::serde_qs::Error),
    #[error("Session Errored out: {0:?}")]
    SessionError(#[from] tiberius_dependencies::axum_database_sessions::SessionError),
    #[error("Error return from Cache Initializer: {0:?}")]
    CacheError(String),
    #[error("Could not parse string as integer: {0:?}")]
    PareInt(#[from] std::num::ParseIntError),

    #[error("ACL Error: {0:?}")]
    ACLError(#[from] tiberius_dependencies::casbin::Error),
}

pub type TiberiusResult<T> = std::result::Result<T, TiberiusError>;

impl axum::response::IntoResponse for TiberiusError {
    fn into_response(self) -> Response {
        match self {
            TiberiusError::AccessDenied => {
                let c = maud::html! {
                    b { (format!("{}", self.to_string())) };
                };
                let c: String = c.into_string();
                let mut hm = HeaderMap::new();
                hm.typed_insert(ContentType::html());
                (StatusCode::FORBIDDEN, hm, c).into_response()
            }
            _ => {
                #[cfg(debug_assert)]
                let c = maud::html! {
                    "Internal Error"
                    br;
                    b { pre { (format!("{}", self.to_string())) } };
                };
                #[cfg(not(debug_assert))]
                let c = {
                    error!("Error presented to user: {:?}", self);
                    maud::html! {
                        "Internal Error"
                        br;
                    }
                };
                let c: String = c.into_string();
                let mut hm = HeaderMap::new();
                hm.typed_insert(ContentType::html());
                (StatusCode::FORBIDDEN, hm, c).into_response()
            }
        }
    }
}
