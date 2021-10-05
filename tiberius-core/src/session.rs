use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use chrono::{Duration, NaiveDateTime, Utc};
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

mod philomena_plug;

#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    client: PgPool,
    table_name: String,
    cookie_name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Session {
    id: Uuid,
    created: NaiveDateTime,
    expires: NaiveDateTime,
    csrf_token: String,
    user_id: Option<i64>,
    data: BTreeMap<String, serde_json::Value>,
    // Indicates if the session structure has been altered, meaning it must be saved to the database
    // Is set automatically if the session is borrowed from SessionPtr as writeable
    #[serde(skip)]
    dirty: bool,
}

impl Session {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn new() -> Self {
        Self {
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
        }
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
        let pss = state.get_db_pool().await;
        let pss = PostgresSessionStore::from_client(pss);
        pss.store_session(self).await.unwrap();
    }
    pub fn set_user(&mut self, user: &User) {
        self.user_id = Some(user.id as i64);
    }
    pub async fn get_user(&self, client: &mut Client) -> TiberiusResult<Option<User>> {
        match self.user_id {
            None => Ok(None),
            Some(user_id) => Ok(User::get_id(client, user_id).await?),
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
    async fn load_session(&self, cookie_value: String) -> TiberiusResult<Option<Session>> {
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

    async fn store_session(&self, session: &Session) -> TiberiusResult<()> {
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

    async fn destroy_session(&self, session: &Session) -> TiberiusResult<()> {
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
        let session: Option<SessionPtr> = req.guard().await.succeeded();

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
            let session = Session::new();
            let session_id = session.id().to_string();
            trace!("New session {}", session_id);
            match self.store_session(&session).await.map_err(|x| {
                warn!("Could not store session: {}", x);
            }) {
                Ok(_) => (),
                Err(e) => warn!("Error in session store (new) : {:?}", e),
            };
            res.adjoin_header(
                Cookie::build(self.cookie_name.clone(), session_id)
                    .path("/")
                    .finish(),
            )
        }
    }
}

type SessionPtrInt = Arc<RwLock<Session>>;

#[derive(Clone)]
pub struct SessionPtr(SessionPtrInt);

impl SessionPtr {
    pub async fn read<'a>(&'a self) -> RwLockReadGuard<'a, Session> {
        self.0.read().await
    }
    pub async fn write<'a>(&'a self) -> RwLockWriteGuard<'a, Session> {
        let mut session = self.0.write().await;
        info!(
            "Session {} marked dirty due to possible write",
            session.id()
        );
        session.mark_dirty();
        session
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionPtr {
    #[instrument(level = "trace", skip(request))]
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session_store: &State<PostgresSessionStore> =
            request.guard().await.expect("no session store");
        let session_id = request.cookies().get(&session_store.cookie_name);
        if let Some(session_id) = session_id {
            let session_data = session_store
                .load_session(session_id.value().to_string())
                .await;
            match session_data {
                Ok(Some(session_data)) => {
                    return Outcome::Success(SessionPtr(Arc::new(RwLock::new(session_data))))
                }
                Ok(None) => info!("Got an empty session"),
                Err(e) => warn!("error trying to get session: {}", e),
            }
        }
        trace!("Couldn't find or load session, generating new session");
        let mut new_session = Session::new();
        new_session.mark_dirty();
        trace!("New session id: {}", new_session.id());
        Outcome::Success(SessionPtr(Arc::new(RwLock::new(new_session))))
    }

    type Error = TiberiusError;
}
