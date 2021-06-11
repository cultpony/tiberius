use anyhow::{Context, Result};
use chrono::Utc;
use erlang_term::Term;
use philomena_models::{Client, Filter, User};
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use tide::{
    sessions::{Session, SessionStore},
    Request,
};

use crate::{app::HTTPReq, state::State};

#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    client: PgPool,
    table_name: String,
}

impl PostgresSessionStore {
    pub fn from_client(client: PgPool) -> Self {
        Self {
            client,
            table_name: "user_sessions".into(),
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
}

#[tide::utils::async_trait]
impl SessionStore for PostgresSessionStore {
    async fn load_session(&self, cookie_value: String) -> Result<Option<tide::sessions::Session>> {
        let id = tide::sessions::Session::id_from_cookie_value(&cookie_value)?;
        let mut conn = self.connection().await?;
        let result: Option<(String,)> = sqlx::query_as(&format!(
            "SELECT session FROM {} WHERE id = $1 AND (expires IS NULL OR expires > $2",
            self.table_name
        ))
        .bind(&id)
        .bind(Utc::now())
        .fetch_optional(&mut conn)
        .await?;
        Ok(result
            .map(|(session,)| serde_json::from_str(&session))
            .transpose()?)
    }

    async fn store_session(&self, session: tide::sessions::Session) -> Result<Option<String>> {
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
        .bind(&session.expiry())
        .execute(&mut conn)
        .await?;
        Ok(session.into_cookie_value())
    }

    async fn destroy_session(&self, session: tide::sessions::Session) -> Result<()> {
        let id = session.id();
        let mut conn = self.connection().await?;
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", self.table_name))
            .bind(&id)
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    async fn clear_store(&self) -> Result<()> {
        let mut conn = self.connection().await?;
        sqlx::query(&format!("TRUNCATE {}", self.table_name))
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}

pub trait SessionExt {
    fn active_filter<'a>(&self, req: &'a HTTPReq) -> Option<&'a Filter>;
}

impl SessionExt for Session {
    fn active_filter<'a>(&self, req: &'a HTTPReq) -> Option<&'a Filter> {
        if let Some(filter) = req.ext::<Filter>() {
            return Some(filter);
        } else {
            return None;
        }
    }
}

pub trait SessionReqExt {
    fn user(&self) -> Option<&User>;
}

impl SessionReqExt for Request<State> {
    fn user(&self) -> Option<&User> {
        self.ext::<User>()
    }
}