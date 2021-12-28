use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use chrono::{Duration, NaiveDateTime, Utc};
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::Cookie,
    request::{FromRequest, Outcome},
    Build, Request, Response, Rocket, State,
};
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use tiberius_models::{Client, User};
use tracing::{info, trace, warn};
use uuid::Uuid;

use crate::state::TiberiusState;
use crate::{
    app::DBPool,
    error::{TiberiusError, TiberiusResult},
};

use crate::session::philomena_plug::handover_session;

pub mod philomena_plug;

#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    client: PgPool,
    table_name: String,
    cookie_name: String,
}

pub trait SessionMode: Copy + Clone + Eq + PartialEq + std::fmt::Debug {}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Authenticated {}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Unauthenticated {}
impl SessionMode for Authenticated {}
impl SessionMode for Unauthenticated {}

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
}

impl Into<Session<Unauthenticated>> for Session<Authenticated> {
    fn into(self) -> Session<Unauthenticated> {
        Session::<Unauthenticated> {
            _type: PhantomData::<Unauthenticated>,
            id: self.id,
            created: self.created,
            expires: self.expires,
            csrf_token: self.csrf_token,
            user_id: None,
            data: self.data,
            dirty: self.dirty,
            ephemeral: self.ephemeral,
            waiting_on_totp: false,
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
        self.expires <= chrono::Utc::now().naive_utc()
    }
    pub fn csrf_token(&self) -> String {
        self.csrf_token.clone()
    }
    pub async fn save(&self, state: &TiberiusState) {
        let pss = state.get_db_pool();
        let pss = PostgresSessionStore::from_client(pss);
        pss.store_session(self).await.unwrap();
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
            .map(|x| serde_json::from_value(x))
            .transpose()?)
    }
    pub fn set_json_data(
        &mut self,
        key: String,
        value: serde_json::Value,
    ) -> Option<serde_json::Value> {
        self.data.insert(key, value)
    }
    pub fn get_json_data(
        &self,
        key: String,
    ) -> Option<&serde_json::Value> {
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
}

impl Session<Authenticated> {
    pub fn new(ephemeral: bool, user_id: i64) -> Self {
        Self {
            _type: PhantomData::<Authenticated>,
            id: Uuid::new_v4(),
            created: chrono::Utc::now().naive_utc(),
            expires: chrono::Utc::now()
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
        }
    }
}

impl Session<Unauthenticated> {
    pub fn new(ephemeral: bool) -> Self {
        Self {
            _type: PhantomData::<Unauthenticated>,
            id: Uuid::new_v4(),
            created: chrono::Utc::now().naive_utc(),
            expires: chrono::Utc::now()
                .naive_utc()
                .checked_add_signed(Duration::days(7))
                .expect("must be valid"),
            data: BTreeMap::new(),
            user_id: None,
            csrf_token: base64::encode(
                ring::rand::generate::<[u8; 32]>(&ring::rand::SystemRandom::new())
                    .unwrap()
                    .expose(),
            ),
            dirty: false,
            ephemeral,
            waiting_on_totp: false,
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
        }
    }
}

impl PostgresSessionStore {
    pub fn from_client(client: PgPool) -> Self {
        Self {
            client,
            table_name: "user_sessions".into(),
            cookie_name: "_tiberius_session".into(),
        }
    }
    async fn connection(&self) -> sqlx::Result<PoolConnection<Postgres>> {
        self.client.acquire().await
    }
    pub async fn cleanup(&self) -> sqlx::Result<()> {
        let mut conn = self.connection().await?;
        sqlx::query(&format!(
            "DELETE FROM {} WHERE expires < $1",
            self.table_name
        ))
        .bind(Utc::now())
        .execute(&mut conn)
        .await?;

        Ok(())
    }
    pub async fn count(&self) -> sqlx::Result<i64> {
        let (count,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", self.table_name))
            .fetch_one(&mut self.connection().await?)
            .await?;
        Ok(count)
    }

    #[instrument(level = "trace", skip(self, cookie_value))]
    async fn load_session<T: SessionMode>(
        &self,
        cookie_value: String,
    ) -> TiberiusResult<Option<Session<T>>> {
        if cookie_value == "" {
            return Ok(None);
        }
        let id: Uuid = cookie_value.parse()?;
        let mut conn = self.connection().await?;
        let result: Option<(String,)> = sqlx::query_as(&format!(
            "SELECT session FROM {} WHERE id = $1 AND (expires IS NOT NULL OR expires > $2)",
            self.table_name
        ))
        .bind(&id)
        .bind(Utc::now())
        .fetch_optional(&mut conn)
        .await?;
        let result = result
            .map(|(session,)| serde_json::from_str(&session))
            .transpose()?;
        trace!("Session: {:?}", result);
        Ok(result)
    }

    async fn store_session<T: SessionMode>(
        &self,
        session: &Session<T>,
    ) -> TiberiusResult<()> {
        if session.ephemeral() {
            return Ok(());
        }
        let id = session.id();
        let string = serde_json::to_string(&session)?;
        let mut conn = self.connection().await?;

        sqlx::query(&format!(
            r#"INSERT INTO {}
            (id, session, expires) SELECT $1, $2, $3
            ON CONFLICT(id) DO UPDATE SET
                expires = EXCLUDED.expires,
                session = EXCLUDED.session"#,
            self.table_name
        ))
        .bind(&id)
        .bind(&string)
        .bind(&session.expires)
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    async fn destroy_session<T: SessionMode>(
        &self,
        session: &Session<T>,
    ) -> TiberiusResult<()> {
        let id = session.id();
        let mut conn = self.connection().await?;
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", self.table_name))
            .bind(&id)
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    async fn clear_store(&self) -> TiberiusResult<()> {
        let mut conn = self.connection().await?;
        sqlx::query(&format!("TRUNCATE {}", self.table_name))
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}

pub struct SessionID(pub Option<Uuid>);

impl Default for SessionID {
    fn default() -> Self {
        Self(None)
    }
}

impl SessionID {
    fn as_str(&self) -> String {
        self.0
            .map(|f| f.to_string())
            .unwrap_or(Uuid::new_v4().to_string())
    }
}

#[rocket::async_trait]
impl Fairing for PostgresSessionStore {
    fn info(&self) -> Info {
        Info {
            name: "Session Middleware",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        Ok(rocket.manage(self.clone()))
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        if req.uri().path().starts_with("/static/") || req.uri().path().starts_with("/favicon") {
            trace!("Skipping Session Handler on Static Asset");
            return;
        }
        trace!("Post Request Session Handler on {}", req.uri().path());
        let session: Option<SessionPtr<Unauthenticated>> =
            req.guard().await.succeeded();
        // todo: handle auth'd sessions

        if let Some(session) = session {
            let session = session.read().await;
            let session_id = session.id().to_string();
            if session.dirty() {
                trace!("Storing session {}, dirty={}", session_id, session.dirty());
                // active sessions are manually saved on change
                match self.store_session(&session).await {
                    Ok(_) => (),
                    Err(e) => warn!("Error in session store (cur) : {}", e),
                };
            }
            res.adjoin_header(
                Cookie::build(self.cookie_name.clone(), session_id)
                    .path("/")
                    .finish(),
            )
        } else {
            trace!("No session in request, making a session");
            let session = if let Some(authorization) = authorization(req) {
                let mut new_session = Session::<Unauthenticated>::new(true);
                match session_from_api_key(&mut new_session, &authorization, req) {
                    Ok(_) => new_session,
                    Err(e) => {
                        warn!("error on ephemeral session: {}", e);
                        Session::<Unauthenticated>::new(false)
                    }
                }
            } else {
                Session::<Unauthenticated>::new(false)
            };
            let session_id = session.id().to_string();
            trace!("New session {}", session_id);
            match self.store_session(&session).await.map_err(|x| {
                warn!("Could not store session: {}", x);
            }) {
                Ok(_) => (),
                Err(e) => warn!("Error in session store (new) : {:?}", e),
            };
            if !session.ephemeral() {
                res.adjoin_header(
                    Cookie::build(self.cookie_name.clone(), session_id)
                        .path("/")
                        .finish(),
                )
            }
        }
    }
}

type SessionPtrInt<T> = Arc<RwLock<Session<T>>>;

#[derive(Clone)]
pub struct SessionPtr<T: SessionMode>(SessionPtrInt<T>);

impl<T: SessionMode> SessionPtr<T> {
    pub async fn read<'a>(&'a self) -> RwLockReadGuard<'a, Session<T>> {
        self.0.read().await
    }
    pub async fn write<'a>(&'a self) -> RwLockWriteGuard<'a, Session<T>> {
        let mut session = self.0.write().await;
        info!(
            "Session {} marked dirty due to possible write",
            session.id()
        );
        session.mark_dirty();
        session
    }
}

// Returns authorization from HTTP Header
fn authorization<'r>(req: &'r Request<'_>) -> Option<String> {
    static HEADER: &str = "Authorization";
    let headers = req.headers();
    if headers.contains(HEADER) {
        let auth_headers: Vec<&str> = headers.get(HEADER).collect();
        if auth_headers.len() == 1 {
            Some(auth_headers[0].to_string())
        } else {
            None
        }
    } else {
        None
    }
}

// Turns an unauthorized session into an authorized session
fn session_from_api_key(
    session: &mut Session<Unauthenticated>,
    key: &str,
    req: &Request<'_>,
) -> TiberiusResult<Session<Authenticated>> {
    todo!()
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionPtr<Unauthenticated> {
    type Error = TiberiusError;

    #[instrument(level = "trace", skip(request))]
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let new_session = |req: &Request<'_>| {
            trace!("Couldn't find or load session, generating new session");
            let mut new_session = if let Some(authorization) = authorization(req) {
                let mut new_session = Session::<Unauthenticated>::new(true);
                match session_from_api_key(&mut new_session, &authorization, req) {
                    Ok(_) => new_session,
                    Err(e) => {
                        warn!("error on ephemeral session: {}", e);
                        Session::<Unauthenticated>::new(false)
                    }
                }
            } else {
                Session::<Unauthenticated>::new(false)
            };
            new_session.mark_dirty();
            trace!("New session id: {}", new_session.id());
            SessionPtr(Arc::new(RwLock::new(new_session)))
        };
        match get_session_ptr(request).await {
            None => Outcome::Success(new_session(request)),
            Some(v) => Outcome::Success(v),
        }
    }
}

/// Makes a SessionPtr if there is a session in the request
async fn get_session_ptr<'r, T: SessionMode>(request: &'r Request<'_>) -> Option<SessionPtr<T>> {
    let session_store: &State<PostgresSessionStore> =
        request.guard().await.expect("no session store");
    let session_id = request.cookies().get(&session_store.cookie_name);
    if let Some(session_id) = session_id {
        let session_data = session_store
            .load_session(session_id.value().to_string())
            .await;
        match session_data {
            Ok(Some(session_data)) => {
                if session_data.more_auth() {
                    return None;
                }
                let session = SessionPtr(Arc::new(RwLock::new(session_data)));
                if let Some(cookie) = request.cookies().get("_philomena_key") {
                    trace!("Philomena session, trying takeover");
                    let state: &State<TiberiusState> = request.guard().await.expect("no tiberius state");
                    let mut client = match state.get_db_client().await {
                        Ok(v) => v,
                        Err(e) => panic!("error in database connection"),
                    };
                    let config = state.config();
                    match handover_session(&mut client, config, cookie.value(), session.clone()).await {
                        Ok(v) => (),
                        Err(e) => {
                            warn!("error: could not handover session: {}", e);
                        }
                    }
                } else {
                    trace!("No philomena session");
                }
                Some(session)
            },
            Ok(None) => {
                info!("Got an empty session");
                return None;
            }
            Err(e) => {
                warn!("error trying to get session: {}", e);
                return None;
            }
        }
    } else {
        return None;
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionPtr<Authenticated> {
    type Error = TiberiusError;

    #[instrument(level = "trace", skip(request))]
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        trace!("Getting SessionPtr<Auth> from request");
        let session = match get_session_ptr(request).await {
            None => {
                trace!("No session in request, skipping handover");
                return Outcome::Forward(())
            },
            Some(v) => v,
        };
        Outcome::Success(session)
    }
}
