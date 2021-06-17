use philomena_models::{ApiKey, Client};
use rocket::{Request, State, form::FromForm};
use sqlx::{pool::PoolConnection, Pool, Postgres};

use crate::{app::DBPool, config::Configuration, error::TiberiusResult, http_client};
use async_trait::async_trait;

pub type DbRef = PoolConnection<Postgres>;

#[derive(serde::Deserialize, Copy, Clone, PartialEq, Eq, rocket::form::FromFormField)]
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
    async fn get_db_pool(&self) -> DBPool;
    async fn get_config(&self) -> &State<Configuration>;
    async fn get_api_key(&self) -> &State<ApiKey>;
    async fn get_db_client(&self) -> TiberiusResult<Client>;
    async fn get_db_client_standalone(pool: DBPool, config: &Configuration) -> TiberiusResult<Client>;
}

#[rocket::async_trait]
impl<'a> SafeSqlxRequestExt for Request<'a> {
    async fn get_db(&self) -> std::result::Result<DbRef, sqlx::Error> {
        let pool = self.get_db_pool().await;
        pool.acquire().await
    }
    async fn get_db_pool(&self) -> DBPool {
        let pool: &State<DBPool> = self.guard().await.succeeded().expect("Site Configuration not loaded");
        pool.inner().clone()
    }
    async fn get_api_key(&self) -> &State<ApiKey> {
        self.guard().await.succeeded().expect("API Key not loaded")
    }
    async fn get_config(&self) -> &State<Configuration> {
        self.guard().await.succeeded().expect("Site Configuration not loaded")
    }
    async fn get_db_client(&self) -> TiberiusResult<Client> {
        Ok(Client::new(
            self.get_db_pool().await,
            &self.get_config().await.search_dir,
        ))
    }
    async fn get_db_client_standalone(pool: DBPool, config: &Configuration) -> TiberiusResult<Client> {
        Ok(Client::new(
            pool,
            &config.search_dir,
        ))
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