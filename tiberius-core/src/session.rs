use std::{
    collections::BTreeMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use async_trait::async_trait;
use axum::headers::{self, Header};
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use tiberius_dependencies::chrono::{Duration, NaiveDateTime, Utc};
use tiberius_dependencies::uuid;
use tiberius_dependencies::{
    async_once_cell::OnceCell,
    axum,
    axum::{
        extract::FromRequest,
        headers::{
            authorization::{Basic, Bearer},
            Authorization, HeaderMapExt,
        },
        http::StatusCode,
        middleware::Next,
    },
    axum_extra::extract::{cookie::Cookie, CookieJar},
    base64,
    base64::Engine,
    http::Request,
};
use tiberius_models::{Client, User};
use tracing::{info, trace, warn};
use uuid::Uuid;

use crate::{
    app::DBPool,
    error::{TiberiusError, TiberiusResult},
    state::TiberiusState, TIBERIUS_SESSION_KEY,
};

use crate::session::philomena_plug::handover_session;

pub mod philomena_plug;

#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    client: PgPool,
    table_name: String,
    cookie_name: String,
}

pub trait SessionMode: Copy + Clone + Eq + PartialEq + std::fmt::Debug + Send {}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Authenticated {}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Unauthenticated {}

#[cfg(test)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Testing {}

impl SessionMode for Authenticated {}
impl SessionMode for Unauthenticated {}

#[cfg(test)]
impl SessionMode for Testing {}

pub enum AuthMethod {
    TOTP,
}

/// Session contains and maintains a user session as well as metadata for the session,
/// such as if the session resulted from a handover or special authorization markers.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Session<MODE: SessionMode> {
    _type: PhantomData<MODE>,
    id: Uuid,
    created: NaiveDateTime,
    expires: NaiveDateTime,
    csrf_token: String,
    user_id: Option<i64>,
    data: BTreeMap<String, serde_json::Value>,
    /// Indicates if the session structure has been altered, meaning it must be saved to the database
    /// Is set automatically if the session is borrowed from SessionPtr as writeable
    #[serde(skip)]
    dirty: bool,
    /// If set to true, the Session is not persisted into the database or sent out to the client via cookie
    /// The main purpose is to handle sessions from bots/API clients
    #[serde(skip)]
    ephemeral: bool,
    waiting_on_totp: bool,

    #[serde(skip, default = "OnceCell::new")]
    cache_user: OnceCell<Option<User>>,
}

impl<T: SessionMode> Clone for Session<T> {
    fn clone(&self) -> Self {
        Self {
            _type: self._type,
            id: self.id,
            created: self.created,
            expires: self.expires,
            csrf_token: self.csrf_token.clone(),
            user_id: self.user_id,
            data: self.data.clone(),
            dirty: self.dirty,
            ephemeral: self.ephemeral,
            waiting_on_totp: self.waiting_on_totp,
            cache_user: OnceCell::new_with(self.cache_user.get().cloned().expect("could not get user from cache")),
        }
    }
}

impl From<Session<Authenticated>> for Session<Unauthenticated> {
    fn from(value: Session<Authenticated>) -> Self {
        Session::<Unauthenticated> {
            _type: PhantomData::<Unauthenticated>,
            id: value.id,
            created: value.created,
            expires: value.expires,
            csrf_token: value.csrf_token,
            user_id: None,
            data: value.data,
            dirty: value.dirty,
            ephemeral: value.ephemeral,
            waiting_on_totp: false,

            cache_user: OnceCell::new(),
        }
    }
}

#[cfg(test)]
impl From<Session<Unauthenticated>> for Session<Testing> {
    fn from(value: Session<Unauthenticated>) -> Session<Testing> {
        Session::<Testing> {
            _type: PhantomData::<Testing>,
            id: value.id,
            created: value.created,
            expires: value.expires,
            csrf_token: value.csrf_token,
            user_id: None,
            data: value.data,
            dirty: value.dirty,
            ephemeral: value.ephemeral,
            waiting_on_totp: false,

            cache_user: OnceCell::new(),
        }
    }
}

#[cfg(test)]
impl From<Session<Authenticated>> for Session<Testing> {
    fn from(value: Session<Authenticated>) -> Session<Testing> {
        Session::<Testing> {
            _type: PhantomData::<Testing>,
            id: value.id,
            created: value.created,
            expires: value.expires,
            csrf_token: value.csrf_token,
            user_id: None,
            data: value.data,
            dirty: value.dirty,
            ephemeral: value.ephemeral,
            waiting_on_totp: false,

            cache_user: OnceCell::new(),
        }
    }
}

impl<T: SessionMode> Session<T> {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn dirty(&self) -> bool {
        self.dirty
    }
    pub fn mark_dirty(&mut self) {
        trace!("Session marked dirty");
        self.dirty = true
    }
    pub fn expiry(&self) -> i64 {
        self.expires.timestamp_millis()
    }
    pub fn expired(&self) -> bool {
        self.expires <= tiberius_dependencies::chrono::Utc::now().naive_utc()
    }
    pub fn csrf_token(&self) -> String {
        self.csrf_token.clone()
    }
    pub fn get_data(&self, key: &str) -> TiberiusResult<Option<String>> {
        Ok(self
            .data
            .get(key)
            .map(|x| serde_json::from_value(x.clone()))
            .transpose()?)
    }
    pub fn set_data(&mut self, key: &str, value: &str) -> TiberiusResult<Option<String>> {
        Ok(self
            .set_json_data(key.to_string(), serde_json::to_value(value)?)
            .map(serde_json::from_value)
            .transpose()?)
    }
    pub fn set_json_data(
        &mut self,
        key: String,
        value: serde_json::Value,
    ) -> Option<serde_json::Value> {
        self.data.insert(key, value)
    }
    pub fn get_json_data(&self, key: String) -> Option<&serde_json::Value> {
        self.data.get(&key)
    }
    /// Returns true if the session is not persisted into cookies or the database backend
    ///
    /// To set a session as ephemeral, it must be created by passing `true` to the `Session::new()` constructor.
    ///
    /// ```
    /// use tiberius_core::session::{Session, Unauthenticated, Authenticated};
    /// let ephemeral_session = Session::<Unauthenticated>::new(true);
    /// let stored_session = Session::<Unauthenticated>::new(false);
    ///
    /// assert!(ephemeral_session.ephemeral());
    /// assert!(!stored_session.ephemeral());
    /// ```
    pub fn ephemeral(&self) -> bool {
        self.ephemeral
    }

    pub async fn get_user(&self, client: &mut Client) -> TiberiusResult<Option<User>> {
        match self.user_id {
            None => Ok(None),
            /*Some(user_id) => Ok(self
            .cache_user
            .get_or_try_init(User::get_id(client, user_id))
            .await?.clone()),*/
            Some(user_id) => Ok(User::get_id(client, user_id).await?),
        }
    }

    pub fn set_user(&mut self, user: &User) {
        self.user_id = Some(user.id as i64);
    }

    pub fn unset_user(&mut self) {
        self.user_id = None;
    }

    /// Indicates that more authentication methods are still being waited on, the session is not yet valid
    pub fn more_auth(&self) -> bool {
        self.waiting_on_totp
    }

    pub fn set_waiting_auths(&mut self, r: AuthMethod) {
        match r {
            AuthMethod::TOTP => self.waiting_on_totp = true,
        }
    }

    pub fn raw_user(&self) -> Option<i64> {
        self.user_id
    }
}

impl Session<Authenticated> {
    pub fn new(ephemeral: bool, user_id: i64) -> Self {
        Self {
            _type: PhantomData::<Authenticated>,
            id: Uuid::new_v4(),
            created: tiberius_dependencies::chrono::Utc::now().naive_utc(),
            expires: tiberius_dependencies::chrono::Utc::now()
                .naive_utc()
                .checked_add_signed(Duration::days(7))
                .expect("must be valid"),
            data: BTreeMap::new(),
            user_id: Some(user_id),
            csrf_token: base64::encode(
                ring::rand::generate::<[u8; 32]>(&ring::rand::SystemRandom::new())
                    .unwrap()
                    .expose(),
            ),
            dirty: false,
            ephemeral,
            waiting_on_totp: false,

            cache_user: OnceCell::new(),
        }
    }
    pub fn into_unauthenticated(self) -> Session<Unauthenticated> {
        Session::<Unauthenticated> {
            _type: PhantomData::<Unauthenticated>,
            id: self.id,
            created: self.created,
            expires: self.expires,
            csrf_token: self.csrf_token,
            user_id: self.user_id,
            data: self.data,
            dirty: self.dirty,
            ephemeral: self.ephemeral,
            waiting_on_totp: false,

            cache_user: OnceCell::new(),
        }
    }
}

impl Session<Unauthenticated> {
    pub fn new(ephemeral: bool) -> Self {
        Self {
            _type: PhantomData::<Unauthenticated>,
            id: Uuid::new_v4(),
            created: tiberius_dependencies::chrono::Utc::now().naive_utc(),
            expires: tiberius_dependencies::chrono::Utc::now()
                .naive_utc()
                .checked_add_signed(Duration::days(7))
                .expect("must be valid"),
            data: BTreeMap::new(),
            user_id: None,
            csrf_token: base64::engine::general_purpose::STANDARD.encode(
                ring::rand::generate::<[u8; 32]>(&ring::rand::SystemRandom::new())
                    .unwrap()
                    .expose(),
            ),
            dirty: false,
            ephemeral,
            waiting_on_totp: false,

            cache_user: OnceCell::new(),
        }
    }
    pub fn into_authenticated(self, user_id: i64) -> Session<Authenticated> {
        Session::<Authenticated> {
            _type: PhantomData::<Authenticated>,
            id: self.id,
            created: self.created,
            expires: self.expires,
            csrf_token: self.csrf_token,
            user_id: Some(user_id),
            data: self.data,
            dirty: self.dirty,
            ephemeral: self.ephemeral,
            waiting_on_totp: false,

            cache_user: OnceCell::new(),
        }
    }
}

#[derive(Default)]
pub struct SessionID(pub Option<Uuid>);

impl SessionID {
    fn as_str(&self) -> String {
        self.0
            .map(|f| f.to_string())
            .unwrap_or(Uuid::new_v4().to_string())
    }
}

// Returns authorization from HTTP Header
fn authorization<B: Send>(req: &Request<B>) -> Option<String> {
    static HEADER: &str = "Authorization";
    let headers = req.headers();
    if headers.get(Authorization::<Bearer>::name()).is_some() {
        let auth_headers: Option<Authorization<Bearer>> =
            headers.typed_get::<headers::Authorization<Bearer>>();
        let auth_headers: Option<Bearer> = auth_headers.map(|x| x.0);
        auth_headers.map(|bearer: Bearer| bearer.token().to_string())
    } else {
        None
    }
}

// Turns an unauthorized session into an authorized session
fn session_from_api_key<B: Send>(
    session: &mut Session<Unauthenticated>,
    key: &str,
    req: &Request<B>,
) -> TiberiusResult<Session<Authenticated>> {
    todo!()
}

pub trait DbSessionExt {
    fn get_session<T: SessionMode>(&self) -> TiberiusResult<Option<Session<T>>>;
    fn set_session<T: SessionMode>(&self, session: Session<T>) -> TiberiusResult<()>;
}

impl DbSessionExt for tiberius_dependencies::tower_sessions::Session {
    fn get_session<T: SessionMode>(&self) -> TiberiusResult<Option<Session<T>>> {
        Ok(self.get(TIBERIUS_SESSION_KEY)?)
    }
    fn set_session<T: SessionMode>(&self, session: Session<T>) -> TiberiusResult<()> {
        Ok(self.insert(TIBERIUS_SESSION_KEY, session)?)
    }
}
