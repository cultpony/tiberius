use std::{path::PathBuf, str::FromStr};

use reqwest::header::HOST;

use crate::session::SessionMode;
use crate::state::TiberiusRequestState;
use crate::{app::DBPool, error::TiberiusResult};

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

#[derive(serde::Deserialize, serde::Serialize, Clone, securefmt::Debug)]
pub struct Configuration {
    pub database_url: url::Url,
    #[serde(default = "default_listen_on")]
    pub listen_on: std::net::SocketAddr,
    #[serde(default = "default_session_cookie")]
    pub session_cookie: String,
    #[serde(default = "default_flash_cookie")]
    pub flash_cookie: String,
    #[serde(default = "default_philomena_encryption_salt", skip_serializing)]
    #[sensitive]
    pub philomena_encryption_salt: String,
    #[serde(default = "default_philomena_signing_salt", skip_serializing)]
    #[sensitive]
    pub philomena_signing_salt: String,
    #[sensitive]
    #[serde(skip_serializing)]
    pub(crate) camo_key: Option<String>,
    pub(crate) camo_host: Option<String>,
    pub(crate) static_host: Option<String>,
    #[serde(default = "default_data_root")]
    pub static_root: String,
    pub cdn_host: Option<String>,
    #[serde(default = "default_image_url_root")]
    pub image_url_root: String,
    pub data_root: Option<std::path::PathBuf>,
    #[serde(alias = "HTTP_PROXY", alias = "HTTPS_PROXY", alias = "SOCKS_PROXY")]
    pub proxy: Option<url::Url>,
    /// Directory from which to load cryptgraphic keys
    /// You can generate keys by using `tiberius gen-keys ./keys`
    /// Default Value: "./keys"
    pub key_directory: Option<std::path::PathBuf>,
    #[serde(
        alias = "TANTIVY_INDEX",
        alias = "INDEX_PATH"
    )]
    pub search_dir: Option<std::path::PathBuf>,
    #[serde(skip_serializing, alias = "PASSWORD_PEPPER")]
    #[sensitive]
    pub password_pepper: Option<String>,
    #[serde(skip_serializing, alias = "PASSWORD_PEPPER")]
    #[sensitive]
    pub(crate) philomena_secret: Option<String>,
    #[serde(skip_serializing, alias = "SESSHO_SECRET")]
    #[sensitive]
    pub(crate) session_handover_secret: Option<String>,
    #[serde(skip_serializing, alias = "STAFF_SECRET")]
    #[sensitive]
    pub(crate) staff_secret: Option<String>,
    #[serde(skip)]
    #[sensitive]
    pub alt_dbconn: Option<DBPool>,
    #[serde(skip_serializing, alias="OTP_SECRET_KEY")]
    #[sensitive]
    pub otp_secret: Option<String>,
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
    pub fn static_host<T: SessionMode>(
        &self,
        rstate: Option<&TiberiusRequestState<'_, T>>,
    ) -> String {
        match rstate {
            Some(v) => self.static_host.as_ref().cloned().unwrap_or(
                v
                    .headers
                    .get_one("host")
                    .unwrap_or("localhost")
                    .to_string(),
            ),
            None => "localhost".to_string(),
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
                std::env::var("OTP_SECRET_KEY").map(|x| x.as_bytes().to_vec()).unwrap_or_default()
            },
        }
    }
    pub fn password_pepper(&self) -> Option<&str> {
        self.password_pepper.as_ref().map(|x| x.as_str())
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            listen_on: std::net::ToSocketAddrs::to_socket_addrs("localhost:8080")
                .unwrap()
                .next()
                .unwrap(),
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
            key_directory: None,
            search_dir: None,
            password_pepper: None,
            philomena_secret: None,
            session_handover_secret: None,
            staff_secret: None,
            alt_dbconn: None,
            otp_secret: None,
        }
    }
}
