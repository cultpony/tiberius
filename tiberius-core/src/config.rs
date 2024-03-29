use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use reqwest::header::HOST;
use tiberius_dependencies::sha3::Digest;
use tiberius_dependencies::{
    axum::headers::{self, HeaderMapExt},
    http::uri::Authority,
};
use tiberius_dependencies::{reqwest, sha3};
use tracing::Level;

use crate::NodeId;
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

impl From<LogLevel> for tracing::Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

#[derive(serde::Deserialize, Clone, securefmt::Debug, clap::Args)]
pub struct Configuration {
    #[clap(env, long, default_value = "warn")]
    pub log_level: LogLevel,
    #[clap(env = "LISTEN_ON", long, default_value = "0.0.0.0:8081")]
    pub bind_to: SocketAddr,
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
    #[serde(skip_serializing, alias = "OTP_SECRET_KEY")]
    #[clap(env, long)]
    #[sensitive]
    pub otp_secret: Option<String>,
    #[clap(long, env = "SENTRY_URL")]
    #[sensitive]
    pub sentry_url: Option<String>,
    /// The ratio of transactions to send to sentry. If not given all transactions are sent.
    /// Value is clamped to the range 0..1 and defaults to 1.0
    #[clap(long, env = "SENTRY_RATIO")]
    pub sentry_ratio: Option<f64>,
    /// The trace sampling rate. If not specified defaults to 0.0
    #[clap(long, env = "SENTRY_TX_RATIO")]
    pub sentry_tx_ratio: Option<f64>,
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
    /// The ID of the current node.
    ///
    /// This must either be a 14 character string with a leading 0x like 0x000000000000 (ie, 6 bytes of data)
    /// or it must be unique per node and will be hashed into 6 bytes.
    #[clap(long)]
    pub node_id: Option<String>,
}

impl Configuration {
    pub async fn db_conn(&self) -> TiberiusResult<DBPool> {
        let opts = sqlx::postgres::PgConnectOptions::from_str(self.database_url.as_str())?
            .application_name(&crate::package_full());
        let conn = sqlx::PgPool::connect_with(opts).await?;

        Ok(conn)
    }
    pub fn camo_config(&self) -> Option<(&String, &String)> {
        self.camo_host
            .as_ref()
            .and_then(|camo_host| self.camo_key.as_ref().map(|camo_key| (camo_host, camo_key)))
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

    /// Sets the Staff Key to be used for accessing Tiberius
    ///
    /// # Safety
    ///
    /// This function must only be called once during startup. Calling this function
    /// outside tests or startup immediately compromises all security guarantees the application makes
    /// regarding access control.
    ///
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
        self.password_pepper.as_deref()
    }

    pub fn image_base(&self) -> PathBuf {
        self.data_root
            .as_ref()
            .expect("image root was needed but not set")
            .join("images")
    }

    /// Return the Node ID. If no node-id is configured, returns the hash of the hostname of the system
    /// and the PID of the current process
    pub fn node_id(&self) -> NodeId {
        match &self.node_id {
            Some(s) if s.starts_with("0x") => {
                todo!("implement raw node id")
            }
            Some(s) => {
                todo!("implement hashed node id")
            }
            None => {
                let hostname = tiberius_dependencies::gethostname();
                let hostname = hostname.to_string_lossy();
                let hostname = hostname.as_bytes();
                let mut pid = std::process::id().to_le_bytes().to_vec();
                let mut full = hostname.to_vec();
                full.append(&mut pid);
                let mut hasher = sha3::Sha3_256::new();
                hasher.update(&full);
                let result = hasher.finalize()[..].to_vec();
                // only grab the first 6 bytes of the hash
                let result = &result[0..6];
                assert!(result.len() == 6);
                let result: [u8; 6] = result.try_into().unwrap();
                NodeId::from(result)
            }
        }
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
            otp_secret: None,
            sentry_url: None,
            sentry_ratio: None,
            sentry_tx_ratio: None,
            log_level: LogLevel::default(),
            bind_to: "127.0.0.1:8081".parse().unwrap(),
            upload_max_size: 104857600,
            rebuild_index_on_startup: false,
            enable_lock_down: false,
            try_use_ondisk_favicon: true,
            node_id: None,
        }
    }
}
