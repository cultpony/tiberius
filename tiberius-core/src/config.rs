use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use reqwest::header::HOST;
use tiberius_dependencies::{
    axum::headers::{self, HeaderMapExt},
    http::uri::Authority,
};

use crate::{
    app::DBPool,
    error::{TiberiusError, TiberiusResult},
    session::SessionMode,
    state::TiberiusRequestState,
};

fn default_data_root() -> String {
    "./res".to_string()
}

fn default_philomena_signing_salt() -> String {
    "signed cookie".to_string()
}

fn default_philomena_encryption_salt() -> String {
    "authenticated encrypted cookie".to_string()
}

fn default_session_cookie() -> String {
    "session".to_string()
}

fn default_flash_cookie() -> String {
    "flash".to_string()
}

fn default_forward_to() -> std::net::SocketAddr {
    std::net::SocketAddr::from_str("localhost:8000").unwrap()
}

fn default_listen_on() -> std::net::SocketAddr {
    std::net::SocketAddr::from_str("localhost:8000").unwrap()
}

fn default_image_url_root() -> String {
    "/img".to_string()
}

#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum LogLevel {
    Error,
    #[default]
    Warn,
    Info,
    Debug,
    Trace,
}

impl FromStr for LogLevel {
    type Err = TiberiusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "err" | "error" => Self::Error,
            "warn" | "default" => Self::Warn,
            "info" => Self::Info,
            "debug" => Self::Debug,
            "trace" | "verbose" => Self::Trace,
            v => return Err(TiberiusError::InvalidLogLevel(v.to_string())),
        })
    }
}

impl Into<tracing::Level> for LogLevel {
    fn into(self) -> tracing::Level {
        use tracing::Level;
        match self {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, securefmt::Debug, clap::Args)]
pub struct Configuration {
    #[clap(env, long, default_value = "warn")]
    pub log_level: LogLevel,
    #[clap(env = "LISTEN_ON", long, default_value = "0.0.0.0:8081")]
    pub bind_to: SocketAddr,
    #[clap(env = "STRANGLE_TO", long)]
    pub strangle_to: Option<url::Url>,
    #[sensitive]
    #[clap(env, long)]
    pub database_url: url::Url,
    #[serde(default = "default_session_cookie")]
    #[clap(env, long, default_value_t = default_session_cookie())]
    pub session_cookie: String,
    #[serde(default = "default_flash_cookie")]
    #[clap(env, long, default_value_t = default_flash_cookie())]
    pub flash_cookie: String,
    #[serde(default = "default_philomena_encryption_salt", skip_serializing)]
    #[clap(env, long, default_value_t = default_philomena_encryption_salt())]
    #[sensitive]
    pub philomena_encryption_salt: String,
    #[serde(default = "default_philomena_signing_salt", skip_serializing)]
    #[clap(env, long, default_value_t = default_philomena_signing_salt())]
    #[sensitive]
    pub philomena_signing_salt: String,
    #[serde(skip_serializing)]
    #[clap(env, long)]
    #[sensitive]
    pub(crate) camo_key: Option<String>,
    #[clap(env, long)]
    pub(crate) camo_host: Option<String>,
    #[clap(env, long)]
    pub(crate) static_host: Option<String>,
    #[serde(default = "default_data_root")]
    #[clap(env, long, default_value_t = default_data_root())]
    pub static_root: String,
    #[clap(env, long)]
    pub cdn_host: Option<String>,
    #[serde(default = "default_image_url_root")]
    #[clap(env, long, default_value_t = default_image_url_root())]
    pub image_url_root: String,
    #[clap(env, long)]
    pub data_root: Option<std::path::PathBuf>,
    #[serde(alias = "HTTP_PROXY", alias = "HTTPS_PROXY", alias = "SOCKS_PROXY")]
    #[clap(env, long)]
    pub proxy: Option<url::Url>,
    #[serde(alias = "TANTIVY_INDEX", alias = "INDEX_PATH")]
    #[clap(env, long)]
    pub search_dir: Option<std::path::PathBuf>,
    #[serde(skip_serializing, alias = "PASSWORD_PEPPER")]
    #[clap(env, long)]
    #[sensitive]
    pub password_pepper: Option<String>,
    #[serde(skip_serializing, alias = "PASSWORD_PEPPER")]
    #[clap(env, long)]
    #[sensitive]
    pub(crate) philomena_secret: Option<String>,
    #[serde(skip_serializing, alias = "SESSHO_SECRET")]
    #[clap(env, long)]
    #[sensitive]
    pub(crate) session_handover_secret: Option<String>,
    #[serde(skip_serializing, alias = "STAFF_SECRET")]
    #[clap(env, long)]
    #[sensitive]
    pub(crate) staff_secret: Option<String>,
    #[serde(skip)]
    #[clap(skip = None)]
    #[sensitive]
    pub alt_dbconn: Option<DBPool>,
    #[serde(skip_serializing, alias = "OTP_SECRET_KEY")]
    #[clap(env, long)]
    #[sensitive]
    pub otp_secret: Option<String>,
    #[clap(long, env = "SENTRY_URL")]
    #[sensitive]
    pub sentry_url: Option<String>,
    /// The ratio of transactions to send to sentry. If not given all transactions are sent.
    /// Value is clamped to the range 0..1
    #[clap(long, env = "SENTRY_RATIO")]
    pub sentry_ratio: Option<f64>,
    #[clap(long, env, default_value = "104857600")]
    pub upload_max_size: u64,
    #[serde(skip_serializing, default)]
    #[clap(long)]
    pub rebuild_index_on_startup: bool,
    /// Requires users to have the site::use ACL entry to gain access
    ///
    /// If false, the middleware regulating access this way is not activated on bootup
    #[clap(long, default_value = "false")]
    pub enable_lock_down: bool,
    /// Will check if the resource folder on disk contains a favicon and use that over the compiled in version if possible
    /// The following path is checked here: /res/favicon.ico
    #[clap(long, default_value = "false")]
    pub try_use_ondisk_favicon: bool,
}

impl Configuration {
    pub async fn db_conn(&self) -> TiberiusResult<DBPool> {
        match &self.alt_dbconn {
            Some(v) => return Ok(v.clone()),
            None => (),
        }
        let opts = sqlx::postgres::PgConnectOptions::from_str(&self.database_url.to_string())?
            .application_name(&crate::package_full());
        let conn = sqlx::PgPool::connect_with(opts).await?;

        Ok(conn)
    }
    pub fn camo_config<'a>(&'a self) -> Option<(&'a String, &'a String)> {
        match &self.camo_host {
            Some(camo_host) => match &self.camo_key {
                Some(camo_key) => Some((camo_host, camo_key)),
                None => None,
            },
            None => None,
        }
    }
    pub fn static_host<T: SessionMode>(&self, rstate: Option<&TiberiusRequestState<T>>) -> String {
        match rstate {
            Some(v) => self.static_host.as_ref().cloned().unwrap_or(
                v.headers
                    .typed_get::<headers::Host>()
                    .unwrap_or(headers::Host::from(Authority::from_static("localhost")))
                    .to_string(),
            ),
            None => match &self.static_host {
                None => "//localhost".to_string(),
                Some(v) => v.to_string(),
            },
        }
    }
    pub fn philomena_secret(&self) -> Option<&String> {
        self.philomena_secret.as_ref()
    }

    pub unsafe fn set_alt_dbconn(&mut self, db: DBPool) {
        self.alt_dbconn = Some(db);
    }
    pub unsafe fn set_staff_key(&mut self, staff_secret: Option<String>) {
        self.staff_secret = staff_secret
    }
    pub fn otp_secret(&self) -> Vec<u8> {
        match &self.otp_secret {
            Some(v) => v.as_bytes().to_vec(),
            None => {
                trace!("Attempting fallback OTP secret loading");
                std::env::var("OTP_SECRET_KEY")
                    .map(|x| x.as_bytes().to_vec())
                    .unwrap_or_default()
            }
        }
    }
    pub fn password_pepper(&self) -> Option<&str> {
        self.password_pepper.as_ref().map(|x| x.as_str())
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            database_url: url::Url::from_str("postgres://localhost/philomena").unwrap(),
            session_cookie: default_session_cookie(),
            flash_cookie: default_flash_cookie(),
            philomena_encryption_salt: default_philomena_encryption_salt(),
            philomena_signing_salt: default_philomena_signing_salt(),
            camo_host: None,
            camo_key: None,
            static_host: None,
            static_root: "./res".to_string(),
            cdn_host: None,
            image_url_root: "/img".to_string(),
            data_root: None,
            proxy: None,
            search_dir: None,
            password_pepper: None,
            philomena_secret: None,
            session_handover_secret: None,
            staff_secret: None,
            alt_dbconn: None,
            otp_secret: None,
            sentry_url: None,
            sentry_ratio: None,
            log_level: LogLevel::default(),
            bind_to: "127.0.0.1:8081".parse().unwrap(),
            strangle_to: None,
            upload_max_size: 104857600,
            rebuild_index_on_startup: false,
            enable_lock_down: false,
            try_use_ondisk_favicon: true,
        }
    }
}
