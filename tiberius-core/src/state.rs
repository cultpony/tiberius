use std::rc::Rc;
use std::time::Instant;
use std::{convert::Infallible, sync::Arc};

use async_std::{fs::File, path::Path};
use casbin::CoreApi;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use rocket::{
    fairing::Fairing,
    fs::NamedFile,
    http::{ContentType, CookieJar, HeaderMap},
    request::{FlashMessage, FromRequest, Outcome},
    response::stream::ReaderStream,
    Request,
};
use tiberius_models::{ApiKey, Client, Conversation, Filter, Notification, SiteNotice, User};

use crate::app::DBPool;
use crate::assets::{AssetLoader, SiteConfig};
use crate::config::Configuration;
use crate::error::{TiberiusError, TiberiusResult};
use crate::footer::FooterData;
use crate::request_helper::DbRef;
use crate::LayoutClass;
use crate::session::{SessionMode, SessionPtr};

#[derive(Clone)]
pub struct TiberiusState {
    pub config: Configuration,
    pub cryptokeys: CryptoKeys,
    pub db_pool: DBPool,
    pub asset_loader: AssetLoader,
    pub client: Client,
    pub casbin: Arc<casbin::Enforcer>,
}

impl TiberiusState {
    pub async fn get_db(&self) -> std::result::Result<DbRef, sqlx::Error> {
        let pool = self.get_db_pool().await;
        pool.acquire().await
    }
    pub fn get_casbin(&self) -> &casbin::Enforcer {
        &self.casbin
    }
    pub async fn get_db_pool(&self) -> DBPool {
        self.db_pool.clone()
    }
    pub async fn get_config(&self) -> &Configuration {
        &self.config
    }
    pub async fn get_db_client(&self) -> TiberiusResult<Client> {
        Ok(self.client.clone())
        /*Ok(Client::new(
            self.get_db_pool().await,
            &self.get_config().await.search_dir,
        ))*/
    }
    #[instrument(level = "trace")]
    pub async fn get_db_client_standalone(
        pool: DBPool,
        config: &Configuration,
    ) -> TiberiusResult<Client> {
        // calling this unnecessarily is bad as it means we loose in-proc cache
        // and locks on data
        warn!("Creating standalone database client");
        Ok(Client::new(pool, &config.search_dir))
    }
}

pub struct TiberiusRequestState<'a, const T: SessionMode> {
    pub cookie_jar: &'a CookieJar<'a>,
    pub headers: &'a HeaderMap<'a>,
    pub uri: &'a rocket::http::uri::Origin<'a>,
    pub session: SessionPtr<T>,
    pub flash: Option<Flash>,
    pub started_at: Instant,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TiberiusRequestState<'r, {SessionMode::Authenticated}> {
    type Error = Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(Self {
            cookie_jar: request.cookies(),
            headers: request.headers(),
            uri: request.uri(),
            session: request.guard::<SessionPtr<{SessionMode::Authenticated}>>().await.succeeded().unwrap(),
            flash: Flash::from_flashm(request.guard::<FlashMessage>().await.succeeded()),
            started_at: Instant::now(),
        })
    }
}

//todo: make this an anonymous session if no auth is needed
#[rocket::async_trait]
impl<'r> FromRequest<'r> for TiberiusRequestState<'r, {SessionMode::Unauthenticated}> {
    type Error = Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(Self {
            cookie_jar: request.cookies(),
            headers: request.headers(),
            uri: request.uri(),
            session: request.guard::<SessionPtr<{SessionMode::Unauthenticated}>>().await.succeeded().unwrap(),
            flash: Flash::from_flashm(request.guard::<FlashMessage>().await.succeeded()),
            started_at: Instant::now(),
        })
    }
}

#[derive(Clone)]
pub struct CryptoKeys {
    pub signing_key: Arc<ring::signature::Ed25519KeyPair>,
    pub random_key: [u8; 64],
}

pub struct EncryptedData<T> {
    pub data: T,
    pub aud: String,
    pub exp: chrono::DateTime<Utc>,
    pub iss: chrono::DateTime<Utc>,
}

impl TiberiusState {
    pub async fn new(config: Configuration) -> TiberiusResult<Self> {
        let cryptokeys = {
            tracing::info!("Loading cryptographic keys");
            let path = config.key_directory.canonicalize()?;
            tracing::debug!("Loading keys from {}", path.display());
            tracing::debug!("Loading ed25519 key");
            let ed25519key = async_std::fs::read(path.join(Path::new("ed25519.pkcs8"))).await?;
            tracing::debug!("Loading main encryption key");
            let randomkeystr = async_std::fs::read(path.join(Path::new("main.key"))).await?;
            assert!(randomkeystr.len() == 64, "Random key must have 64 bytes");
            let ed25519key = ring::signature::Ed25519KeyPair::from_pkcs8(&ed25519key)?;
            tracing::debug!("Loading encryption keys complete");
            let mut randomkey: [u8; 64] = [0; 64];
            for char in 0..64 {
                randomkey[char] = randomkeystr[char];
            }
            CryptoKeys {
                signing_key: Arc::new(ed25519key),
                random_key: randomkey,
            }
        };
        tracing::debug!("Grabbing Database Pool for HTTP Stateful Requests");
        let db_pool = config.db_conn().await?;
        let casbin_adapter = sqlx_adapter::SqlxAdapter::new_with_pool(db_pool.clone()).await?;
        let casbin_model = casbin::DefaultModel::from_str(include_str!("../casbin.ini")).await?;
        let casbin = Arc::new(casbin::Enforcer::new(casbin_model, casbin_adapter).await?);
        Ok(Self {
            config: config.clone(),
            client: Client::new(db_pool.clone(), &config.search_dir),
            cryptokeys,
            asset_loader: AssetLoader::new(&config)?,
            casbin,
            db_pool,
        })
    }
    pub fn config(&self) -> &Configuration {
        &self.config
    }

    /// The data will be signed and encrypted securely
    /// The resulting string can be freely sent to the user without being able to inspect the data itself
    pub fn encrypt_data<T: serde::ser::Serialize>(&self, data: &T) -> TiberiusResult<String> {
        let msg = serde_cbor::to_vec(data)?;
        let msg = base64::encode(msg);
        let footer = "";
        Ok(
            paseto::v2::local_paseto(&msg, Some(footer), &self.cryptokeys.random_key)
                .map_err(|e| TiberiusError::Paseto(e.to_string()))?,
        )
    }
    pub fn decrypt_data<T: serde::de::DeserializeOwned, S: Into<String>>(&self, data: S) -> T {
        let data: String = data.into();
        todo!()
    }
    pub fn site_config(&self) -> &SiteConfig {
        self.asset_loader.site_config()
    }
    pub fn footer_data(&self) -> &FooterData {
        self.asset_loader.footer_data()
    }
    pub fn site_notices(&self) -> Option<SiteNotices> {
        Some(SiteNotices(vec![SiteNotice {
            id: 0,
            title: "TestBoard".to_string(),
            text: "Tiberius is still in development, please report us any bugs and mind the gap!"
                .to_string(),
            link: None,
            link_text: None,
            live: true,
            start_date: NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1),
            finish_date: chrono::Utc::now().naive_utc(),
            created_at: NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1),
            updated_at: NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1),
            user_id: 0,
        }]))
    }
}

#[rocket::async_trait]
pub trait StateRequestExt {
    async fn state<'a>(&'a self) -> &'a TiberiusState;
}

#[rocket::async_trait]
impl<'a> StateRequestExt for Request<'a> {
    async fn state<'b>(&'b self) -> &'b TiberiusState {
        let state = self.guard().await;
        let state: &rocket::State<TiberiusState> = state.succeeded().unwrap();
        state.inner()
    }
}

impl<'a, const T: SessionMode> TiberiusRequestState<'a, T> {
    pub async fn flash(&self) -> Option<Flash> {
        self.flash.clone()
    }
    pub async fn theme_name(&self, state: &TiberiusState) -> TiberiusResult<String> {
        let user = self.user(state).await?;
        Ok(if let Some(user) = user {
            user.theme.clone()
        } else {
            "default".to_string()
        })
    }
    pub async fn user(&self, state: &TiberiusState) -> TiberiusResult<Option<User>> {
        Ok(self
            .session
            .read().await
            .get_user(&mut state.get_db_client().await?)
            .await?)
    }
    pub async fn filter(&self, state: &TiberiusState) -> TiberiusResult<Filter> {
        let mut client = state.get_db_client().await?;
        if let Some(user) = self.user(state).await? {
            if let Some(current_filter_id) = user.current_filter_id {
                if let Some(filter) = Filter::get_id(&mut client, current_filter_id as i64).await? {
                    return Ok(filter);
                }
            }
        }
        Ok(Filter::default_filter(&mut client).await?)
    }
}

impl<'a, const T: SessionMode> TiberiusRequestState<'a, T> {
    pub async fn search_query(&self) -> TiberiusResult<String> {
        Ok("".to_string()) // TODO: recover search query
    }
    pub async fn conversations(&self) -> TiberiusResult<Vec<Conversation>> {
        Ok(Vec::new()) //TODO: grab user notifications
    }
    pub async fn notifications(&self) -> TiberiusResult<Vec<Notification>> {
        Ok(Vec::new()) //TODO: grab user notifications
    }
    pub async fn layout_class(&self) -> LayoutClass {
        // TODO: let user set LayoutClass
        LayoutClass::Narrow
    }
    pub async fn csd_extra(&self) -> TiberiusResult<ClientSideExtra> {
        // TODO: set Extra Client Side Data here
        Ok(ClientSideExtra::new())
    }
    pub async fn interactions(&self) -> TiberiusResult<Interactions> {
        // TODO: load user interactions
        Ok(Vec::new())
    }
}

pub type ClientSideExtra = std::collections::BTreeMap<String, serde_json::Value>;
pub type Interactions = Vec<()>;

pub struct SiteNotices(pub Vec<SiteNotice>);

impl Default for SiteNotices {
    fn default() -> Self {
        Self(Vec::new())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum Flash {
    Info(String),
    Alert(String),
    Error(String),
    Warning(String),
    None,
}

impl Flash {
    pub fn error<S: Into<String>>(e: S) -> Flash {
        Self::Error(e.into())
    }
    pub fn alert<S: Into<String>>(a: S) -> Flash {
        Self::Alert(a.into())
    }
    pub fn warning<S: Into<String>>(w: S) -> Flash {
        Self::Warning(w.into())
    }
    pub fn info<S: Into<String>>(i: S) -> Flash {
        Self::Info(i.into())
    }

    fn kind(&self) -> String {
        match self {
            Self::Info(_) => "info",
            Self::Alert(_) => "alert",
            Self::Warning(_) => "warning",
            Self::Error(_) => "error",
            Self::None => "none",
        }
        .to_string()
    }

    fn message(&self) -> String {
        match self {
            Self::Info(v) => v.clone(),
            Self::Alert(v) => v.clone(),
            Self::Warning(v) => v.clone(),
            Self::Error(v) => v.clone(),
            Self::None => "none".to_string(),
        }
    }

    pub fn into_resp<'r, 'o, T>(self, r: T) -> rocket::response::Flash<T>
    where
        'o: 'r,
        T: rocket::response::Responder<'r, 'o>,
    {
        rocket::response::Flash::new(r, self.kind(), self.message())
    }

    pub fn from_flashm(fm: Option<rocket::request::FlashMessage>) -> Option<Self> {
        match fm {
            Some(fm) => Some(match fm.kind() {
                "info" => Flash::Info(fm.message().to_string()),
                "alert" => Flash::Alert(fm.message().to_string()),
                "warning" => Flash::Warning(fm.message().to_string()),
                "error" => Flash::Error(fm.message().to_string()),
                "none" => Self::None,
                _ => Self::None,
            }),
            None => None,
        }
    }
}

impl Default for Flash {
    fn default() -> Self {
        Self::None
    }
}
