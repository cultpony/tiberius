use std::convert::Infallible;
use std::io::Cursor;
use rocket::response::status;
use rocket::{http::Status, response::Responder, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TiberiusError {
    #[error("Database Error: {0}")]
    Database(#[from] philomena_models::PhilomenaModelError),
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
}

pub type TiberiusResult<T> = std::result::Result<T, TiberiusError>;

impl<'r> Responder<'r, 'static> for TiberiusError {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let c = maud::html! {
            "Internal Error"
            p { (format!("{}", self.to_string())) };
        };
        let c: String = c.into_string();
        Ok(Response::build()
            .status(Status::InternalServerError)
            .sized_body(c.bytes().len(), Cursor::new(c))
            .finalize())
    }
}
