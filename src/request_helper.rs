use anyhow::Result;
use philomena_models::{ApiKey, Client};
use sqlx::{pool::PoolConnection, Pool, Postgres};

use crate::{
    app::DBPool,
    config::Configuration,
    http_client,
    state::{AuthToken, State},
};
use async_trait::async_trait;

pub type DbRef = PoolConnection<Postgres>;

#[derive(serde::Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum FormMethod {
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "update")]
    Update,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
#[serde(transparent)]
pub struct CSRFToken(String);

impl Into<String> for CSRFToken {
    fn into(self) -> String {
        self.0
    }
}

#[derive(serde::Deserialize, rocket::form::FromForm)]
pub struct ApiFormData<T> {
    #[serde(rename = "_csrf_token")]
    csrf_token: CSRFToken,
    #[serde(rename = "_method")]
    method: Option<FormMethod>,
    #[serde(flatten, bound(deserialize = "T: serde::Deserialize<'de>"))]
    pub data: T,
}

impl<T> ApiFormData<T> {
    pub fn verify_csrf(&self, method: Option<FormMethod>) -> bool {
        // verify method expected == method gotten
        if method != self.method {
            return false
        }
        //TODO: verify CSRF valid!
        true
    }
    pub fn method(&self) -> Option<FormMethod> {
        self.method
    }
}

#[async_trait]
pub trait SafeSqlxRequestExt {
    /// Caller must ensure they drop the database!
    async fn get_db(&self) -> std::result::Result<DbRef, sqlx::Error>;
    fn get_api_key(&self) -> ApiKey;
    async fn get_db_client(&self) -> Result<Client>;
    async fn get_db_client_standalone(pool: DBPool, config: &Configuration) -> Result<Client>;
}

/*
#[async_trait]
impl SafeSqlxRequestExt for tide::Request<State> {
    async fn get_db<'b>(&'b self) -> std::result::Result<DbRef, sqlx::Error> {
        let opt = self.ext::<ConnectionWrapper>();
        let opt = opt.expect("needed database but not injected");
        Ok(opt.pool.acquire().await?)
    }
    fn get_api_key(&self) -> ApiKey {
        self.ext::<AuthToken>()
            .and_then(|x: &AuthToken| Some(x.clone().into()))
            .unwrap_or(ApiKey::new(None))
    }
    async fn get_db_client(&self) -> Result<Client> {
        Ok(Client::new(
            self.get_db().await?,
            http_client(self.state().config())?,
            Some(self.state().config().forward_to.to_string()),
            "http".to_string(),
            self.get_api_key(),
        ))
    }
    async fn get_db_client_standalone(pool: DBPool, config: &Configuration) -> Result<Client> {
        Ok(Client::new(
            pool.acquire().await?,
            http_client(config)?,
            None,
            "http".to_string(),
            ApiKey::new(None),
        ))
    }
}*/

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

/*#[tide::utils::async_trait]
impl Middleware<State> for SqlxMiddleware {
    async fn handle(
        &self,
        mut req: tide::Request<State>,
        next: tide::Next<'_, State>,
    ) -> tide::Result {
        if req.ext::<ConnectionWrapper>().is_some() {
            return Ok(next.run(req).await);
        }
        let cw = ConnectionWrapper {
            pool: self.pool.clone(),
        };
        req.set_ext(cw);

        Ok(next.run(req).await)
    }
}
*/