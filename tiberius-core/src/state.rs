use std::{
    borrow::BorrowMut,
    cell::RefCell,
    convert::Infallible,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::{IntoResponse, Redirect},
    RequestExt, RequestPartsExt,
};
use tiberius_dependencies::{casbin::CoreApi, *};

use async_std::{fs::File, path::Path, sync::RwLock};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use maud::{Markup, PreEscaped};
use sqlx::{Pool, Postgres};
use tiberius_dependencies::{
    async_once_cell::OnceCell,
    axum::{extract::FromRequest, http::status::StatusCode},
    axum_csrf::CsrfToken,
    axum_flash::{Flash, IncomingFlashes},
};
use tiberius_models::{ApiKey, Client, Conversation, Filter, Notification, SiteNotice, User};
use tokio::sync::Mutex;

use crate::acl::{verify_acl, ACLActionSite, ACLObject};
use crate::{
    app::DBPool,
    assets::{AssetLoader, SiteConfig},
    config::Configuration,
    error::{TiberiusError, TiberiusResult},
    footer::FooterData,
    request_helper::DbRef,
    session::{Authenticated, DbSessionExt, Session, SessionMode, Unauthenticated},
    *,
};

use anyhow::Context;

#[derive(Clone, Debug)]
pub struct UrlDirections {
    pub login_page: axum::http::Uri,
}

#[derive(Clone)]
pub struct TiberiusState {
    pub config: Configuration,
    pub db_pool: DBPool,
    pub asset_loader: AssetLoader,
    pub client: Client,
    pub url_directions: Arc<UrlDirections>,
    /// used for rendering out caches
    pub page_subtext_cache: moka::future::Cache<PageSubtextCacheTag, PreEscaped<String>>,
    pub csd_cache: moka::future::Cache<u64, Markup>,
    pub comment_cache: moka::future::Cache<u64, Markup>,
    pub csrf: axum_csrf::CsrfConfig,
    pub flash: axum_flash::Config,
    pub csp: CSPHeader,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSubtextCacheTag {
    /// Cache Key for Staff Page with User Id
    StaffPageContent { logged_in: bool, user: i32 },
}

impl PageSubtextCacheTag {
    pub fn staff_page_content(user: &Option<User>) -> Self {
        match user {
            Some(user) => PageSubtextCacheTag::StaffPageContent {
                logged_in: true,
                user: user.id,
            },
            None => PageSubtextCacheTag::StaffPageContent {
                logged_in: false,
                user: 0,
            },
        }
    }
}

impl std::fmt::Debug for TiberiusState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TiberiusState")
            .field("config", &self.config)
            .field("asset_loader", &self.asset_loader)
            .finish()
    }
}

impl TiberiusState {
    pub async fn get_db(&self) -> std::result::Result<DbRef, sqlx::Error> {
        let pool = self.get_db_pool();
        pool.acquire().await
    }
    /// Returns the Staff Only Key if set, otherwise None
    pub fn staff_only(&self) -> Option<String> {
        self.get_config().staff_secret.clone()
    }
    pub fn get_db_pool(&self) -> DBPool {
        self.db_pool.clone()
    }
    pub async fn get_acl_enforcer(&self) -> TiberiusResult<casbin::Enforcer> {
        let client = self.get_db_pool();
        let casbin_model = casbin::DefaultModel::from_str(include_str!("../casbin.ini"))
            .await
            .expect("invalid ACL model, cannot build casbin enforcer");
        let adapter = sqlx_adapter::SqlxAdapter::new_with_pool(client).await?;
        Ok(casbin::Enforcer::new(casbin_model, adapter).await?)
    }
    pub fn get_config(&self) -> &Configuration {
        &self.config
    }
    #[instrument]
    pub fn get_db_client(&self) -> Client {
        self.client.clone()
    }
    #[instrument(skip(pool, config))]
    pub async fn get_db_client_standalone(
        pool: DBPool,
        config: &Configuration,
    ) -> TiberiusResult<Client> {
        // calling this unnecessarily is bad as it means we loose in-proc cache
        // and locks on data
        warn!("Creating standalone database client");
        Ok(Client::new(pool, config.search_dir.as_ref()))
    }

    pub fn csp(&self) -> CSPHeader {
        self.csp.clone()
    }
}

impl FromRef<&TiberiusState> for axum_flash::Config {
    fn from_ref(state: &&TiberiusState) -> Self {
        state.flash.clone()
    }
}

impl FromRef<&TiberiusState> for axum_csrf::CsrfConfig {
    fn from_ref(input: &&TiberiusState) -> Self {
        input.csrf.clone()
    }
}

impl FromRef<TiberiusState> for axum_flash::Config {
    fn from_ref(state: &TiberiusState) -> Self {
        state.flash.clone()
    }
}

impl FromRef<TiberiusState> for axum_csrf::CsrfConfig {
    fn from_ref(input: &TiberiusState) -> Self {
        input.csrf.clone()
    }
}

pub struct TiberiusRequestState<T: SessionMode> {
    pub cookie_jar: axum_extra::extract::cookie::CookieJar,
    pub uri: axum::extract::OriginalUri,
    session: Session<T>,
    db_session: tower_sessions::Session,
    pub headers: axum::http::HeaderMap,
    pub incoming_flashes: IncomingFlashes,
    pub started_at: Instant,

    cache_filter: OnceCell<Filter>,

    csrf_token: axum_csrf::CsrfToken,
}

impl<T> IntoResponse for TiberiusRequestState<T>
where
    T: SessionMode,
{
    fn into_response(self) -> axum::response::Response {
        self.cookie_jar.into_response()
    }
}

#[cfg(test)]
impl TiberiusRequestState<session::Testing> {
    #[allow(clippy::diverging_sub_expression)]
    pub async fn default() -> Self {
        let request = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let state: TiberiusState = todo!();
        let self1: TiberiusRequestState<Unauthenticated> =
            TiberiusRequestState::from_request(request, &state)
                .await
                .unwrap();
        self1.into_testing()
    }
}

impl std::fmt::Debug for TiberiusRequestState<Authenticated> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TiberiusRequestState")
            .field("authenticated", &true)
            .field("headers", &self.headers)
            .field("uri", &self.uri)
            .field("started_at", &self.started_at)
            .finish()
    }
}

impl std::fmt::Debug for TiberiusRequestState<Unauthenticated> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TiberiusRequestState")
            .field("authenticated", &false)
            .field("headers", &self.headers)
            .field("uri", &self.uri)
            .field("started_at", &self.started_at)
            .finish()
    }
}

impl<A: SessionMode> TiberiusRequestState<A> {
    fn verify_staff_header(
        req: &Parts,
        state: &TiberiusState,
    ) -> Result<(), axum::response::Response> {
        match state.staff_only() {
            None => Ok(()),
            Some(v) => {
                let hdr = req
                    .headers
                    .get("X-Tiberius-Staff-Auth")
                    .map(|x| x.to_str())
                    .transpose();
                let hdr = hdr.map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Could not read staff key as valid utf8 string",
                    )
                        .into_response()
                })?;
                let hdr = hdr.map(|x| x.as_bytes()).unwrap_or_default();
                let is_eq = ring::constant_time::verify_slices_are_equal(hdr, v.as_bytes()).is_ok();
                if !is_eq {
                    debug!("No staff key, denying access");
                    Err((
                        StatusCode::UNAUTHORIZED,
                        "Staff-Only Mode Enabled, lacking Staff Key",
                    )
                        .into_response())
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn csrf_token(&self) -> &CsrfToken {
        &self.csrf_token
    }
}

#[cfg(test)]
impl TiberiusRequestState<Unauthenticated> {
    pub fn into_testing(self) -> TiberiusRequestState<session::Testing> {
        TiberiusRequestState::<session::Testing> {
            cookie_jar: self.cookie_jar,
            uri: self.uri,
            session: self.session.into(),
            db_session: self.db_session,
            headers: self.headers,
            incoming_flashes: self.incoming_flashes,
            started_at: self.started_at,
            cache_filter: self.cache_filter,
            csrf_token: self.csrf_token,
        }
    }
}

#[cfg(test)]
impl TiberiusRequestState<Authenticated> {
    pub fn into_testing(self) -> TiberiusRequestState<session::Testing> {
        TiberiusRequestState::<session::Testing> {
            cookie_jar: self.cookie_jar,
            uri: self.uri,
            session: self.session.into(),
            db_session: self.db_session,
            headers: self.headers,
            incoming_flashes: self.incoming_flashes,
            started_at: self.started_at,
            cache_filter: self.cache_filter,
            csrf_token: self.csrf_token,
        }
    }
}

#[async_trait]
impl FromRequestParts<TiberiusState> for TiberiusRequestState<Authenticated> {
    type Rejection = Response;
    async fn from_request_parts(
        req: &mut Parts,
        state: &TiberiusState,
    ) -> Result<Self, Self::Rejection> {
        debug!("Checking out Authenticated Request State");
        Self::verify_staff_header(req, state).map_err(|e| e.into_response())?;
        let db_session: tower_sessions::Session =
            req.extract()
                .await
                .map_err(|e: (StatusCode, &'static str)| e.into_response())?;
        let session: Session<Authenticated> = db_session
            .get(TIBERIUS_SESSION_KEY)
            .map_err(|e| todo!())?
            .unwrap_or_else(|| todo!());
        let headers = req.headers.clone();
        let rstate = Self {
            cookie_jar: req
                .extract()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, "Could not load cookies"))
                .map_err(|e| e.into_response())?,
            uri: req
                .extract()
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Could not load original URI",
                    )
                })
                .map_err(|e| e.into_response())?,
            session,
            db_session,
            headers,
            started_at: Instant::now(),
            incoming_flashes: IncomingFlashes::from_request_parts(req, &state)
                .await
                .map_err(|e: (StatusCode, &'static str)| e.into_response())?,
            cache_filter: OnceCell::new(),
            csrf_token: CsrfToken::from_request_parts(req, &state)
                .await
                .map_err(|e: (StatusCode, &'static str)| e.into_response())?,
        };
        if state.config().enable_lock_down
            && !verify_acl(state, &rstate, ACLObject::Site, ACLActionSite::Use)
                .await
                .unwrap_or(false)
        {
            return Err(TiberiusError::AccessDenied.into_response());
        }
        Ok(rstate)
    }
}

#[async_trait]
impl FromRequestParts<TiberiusState> for TiberiusRequestState<Unauthenticated> {
    type Rejection = (Flash, Response);

    async fn from_request_parts(
        req: &mut Parts,
        state: &TiberiusState,
    ) -> Result<Self, Self::Rejection> {
        debug!("Checking out Unauthenticated Request State");
        let flash = Flash::from_request_parts(req, state)
            .await
            .expect("flash unwrap is infallible");
        Self::verify_staff_header(req, state).map_err(|e| (flash.clone(), e))?;
        let allow_unauthenticated = req
            .extensions
            .get::<TiberiusState>()
            .map(|x| !x.config().enable_lock_down)
            .unwrap_or(false);
        let db_session: tower_sessions::Session =
            req.extract()
                .await
                .map_err(|e: (StatusCode, &'static str)| (flash.clone(), e.into_response()))?;
        let session: Session<Unauthenticated> = if allow_unauthenticated {
            db_session.get(TIBERIUS_SESSION_KEY).map_err(|err| {
                todo!()
            })?
                .unwrap_or_else(|| Session::<Unauthenticated>::new(false))
        } else {
            let authed_session: Option<Session<Authenticated>> = db_session.get(TIBERIUS_SESSION_KEY).map_err(|err| {
                todo!()
            })?;
            match authed_session {
                Some(s) => s.into_unauthenticated(),
                None => {
                    let uri = state.url_directions.login_page.clone();
                    if req.uri != uri {
                        return Err((
                            flash.error("You must login to access this website"),
                            Redirect::temporary(uri.to_string().as_str()).into_response(),
                        ));
                    } else {
                        Session::<Unauthenticated>::new(false)
                    }
                }
            }
        };
        let headers = req.headers.clone();
        let rstate = Self {
            cookie_jar: req.extract().await.map_err(|e| {
                (
                    flash.clone(),
                    (StatusCode::INTERNAL_SERVER_ERROR, "Could not load cookies").into_response(),
                )
            })?,
            uri: req.extract().await.map_err(|e| {
                (
                    flash.clone(),
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Could not load original URI",
                    )
                        .into_response(),
                )
            })?,
            session,
            db_session,
            headers,
            started_at: Instant::now(),
            incoming_flashes: IncomingFlashes::from_request_parts(req, &state)
                .await
                .map_err(|e: (StatusCode, &'static str)| (flash.clone(), e.into_response()))?,
            cache_filter: OnceCell::new(),
            csrf_token: CsrfToken::from_request_parts(req, &state)
                .await
                .map_err(|e: (StatusCode, &'static str)| (flash.clone(), e.into_response()))?,
        };
        if state.config().enable_lock_down {
            let uri = state.url_directions.login_page.clone();
            if req.uri != uri {
                // TODO: we should handle errors here but for the ACL it doesn't matter that much
                if !verify_acl(state, &rstate, ACLObject::Site, ACLActionSite::Use)
                    .await
                    .unwrap_or(false)
                {
                    return Err((flash, TiberiusError::AccessDenied.into_response()));
                }
            }
        }
        Ok(rstate)
    }
}

impl TiberiusState {
    #[instrument(skip(config, csrf, flash, csp))]
    pub async fn new(
        config: Configuration,
        url_dirs: UrlDirections,
        csrf: axum_csrf::CsrfConfig,
        flash: axum_flash::Config,
        csp: CSPHeader,
    ) -> TiberiusResult<Self> {
        tracing::debug!("Grabbing Database Pool for HTTP Stateful Requests");
        let db_pool = config
            .db_conn()
            .await
            .context("db connection could not be established")?;
        Ok(Self {
            url_directions: Arc::new(url_dirs),
            config: config.clone(),
            client: Client::new(db_pool.clone(), config.search_dir.as_ref()),
            asset_loader: AssetLoader::new(&config).context("asset loader failed")?,
            db_pool,
            page_subtext_cache: moka::future::Cache::builder()
                .max_capacity(PAGE_SUBTEXT_CACHE_SIZE)
                .initial_capacity(PAGE_SUBTEXT_CACHE_START_SIZE)
                .time_to_live(PAGE_SUBTEXT_CACHE_TTL)
                .time_to_idle(PAGE_SUBTEXT_CACHE_TTI)
                .build(),
            csd_cache: moka::future::Cache::builder()
                .max_capacity(CSD_CACHE_SIZE)
                .initial_capacity(CSD_CACHE_START_SIZE)
                .time_to_live(CSD_CACHE_TTL)
                .time_to_idle(CSD_CACHE_TTI)
                .build(),
            comment_cache: moka::future::Cache::builder()
                .max_capacity(COMMENT_CACHE_SIZE)
                .initial_capacity(COMMENT_CACHE_START_SIZE)
                .time_to_live(COMMENT_CACHE_TTL)
                .time_to_idle(COMMENT_CACHE_TTI)
                .build(),
            csrf,
            flash,
            csp,
        })
    }
    pub fn config(&self) -> &Configuration {
        &self.config
    }
    #[instrument]
    pub fn site_config(&self) -> &SiteConfig {
        self.asset_loader.site_config()
    }
    #[instrument]
    pub fn footer_data(&self) -> &FooterData {
        self.asset_loader.footer_data()
    }
    pub async fn system_filters(&self) -> TiberiusResult<Vec<Filter>> {
        Ok(Filter::get_system(&mut self.get_db_client()).await?)
    }
    pub async fn site_notices(&self) -> TiberiusResult<SiteNotices> {
        let mut client = self.get_db_client();
        let mut notices = SiteNotice::get_all_active_notices(&mut client).await?;
        notices.push(SiteNotice {
            id: 0,
            title: "Notice".to_string(),
            text: "Tiberius is still in development, please report us any bugs and mind the gap!"
                .to_string(),
            link: String::default(),
            link_text: String::default(),
            live: true,
            start_date: NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1),
            finish_date: chrono::Utc::now().naive_utc(),
            created_at: NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1),
            updated_at: NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1),
            user_id: 0,
        });
        Ok(SiteNotices(notices))
    }
}

impl<T: SessionMode> TiberiusRequestState<T> {
    pub fn session(&self) -> &Session<T> {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session<T> {
        &mut self.session
    }

    /// Required to persist session data changes
    pub fn push_session_update(&mut self) -> TiberiusResult<()> {
        Ok(self.db_session.insert(TIBERIUS_SESSION_KEY, self.session.clone())?)
    }

    pub fn db_session_mut(
        &mut self,
    ) -> &mut tower_sessions::Session {
        &mut self.db_session
    }

    #[instrument(skip(self, state))]
    pub async fn theme_name(&self, state: &TiberiusState) -> TiberiusResult<String> {
        let user = self.user(state).await?;
        Ok(if let Some(user) = user {
            user.user_settings.theme
        } else {
            "default".to_string()
        })
    }
    #[instrument(skip(self, state))]
    pub async fn user(&self, state: &TiberiusState) -> TiberiusResult<Option<User>> {
        self.session.get_user(&mut state.get_db_client()).await
    }

    pub async fn filter<'a>(&'a self, state: &'a TiberiusState) -> TiberiusResult<&'a Filter> {
        self.cache_filter
            .get_or_try_init(self.int_filter(state))
            .await
    }

    #[instrument(skip(self, state))]
    async fn int_filter(&self, state: &TiberiusState) -> TiberiusResult<Filter> {
        let mut client = state.get_db_client();
        if let Some(user) = self.user(state).await? {
            if let Some(current_filter_id) = user.user_settings.current_filter_id {
                if let Some(filter) = Filter::get_id(&mut client, current_filter_id as i64).await? {
                    return Ok(filter);
                }
            }
        }
        Ok(Filter::default_filter(&mut client).await?)
    }
}

impl<T: SessionMode> TiberiusRequestState<T> {
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
        LayoutClass::Wide
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

#[derive(Default)]
pub struct SiteNotices(pub Vec<SiteNotice>);
