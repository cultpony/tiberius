use std::{path::PathBuf, str::FromStr};

use reqwest::header::HOST;

use crate::{app::{DBPool, HTTPReq}, error::TiberiusResult};

fn default_data_root() -> String {
    "./res".to_string()
}

fn default_postgres_port() -> u16 {
    5432
}

fn default_postgres_host() -> String {
    "localhost".to_string()
}

fn default_postgres_user() -> String {
    "philomena".to_string()
}

fn default_postgres_password() -> String {
    "philomena".to_string()
}

fn default_philomena_signing_salt() -> String {
    "signed cookie".to_string()
}

fn default_philomena_encryption_salt() -> String {
    "authenticated encrypted cookie".to_string()
}

fn default_session_cookie() -> String {
    "_philomena_key".to_string()
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

fn default_key_directory() -> std::path::PathBuf {
    let path = std::path::PathBuf::from_str("./keys").unwrap();
    path.canonicalize()
        .expect("require a keys directory to start")
}

#[derive(serde::Deserialize, serde::Serialize, Clone, securefmt::Debug)]
pub struct Configuration {
    #[serde(default = "default_listen_on")]
    pub listen_on: std::net::SocketAddr,
    #[serde(default = "default_forward_to")]
    pub forward_to: std::net::SocketAddr,
    #[serde(default = "default_session_cookie")]
    pub session_cookie: String,
    #[serde(default = "default_philomena_encryption_salt", skip_serializing)]
    #[sensitive]
    pub philomena_encryption_salt: String,
    #[serde(default = "default_philomena_signing_salt", skip_serializing)]
    #[sensitive]
    pub philomena_signing_salt: String,
    #[serde(default = "default_postgres_host")]
    pub postgres_host: String,
    #[serde(default = "default_postgres_port")]
    pub postgres_port: u16,
    #[serde(default = "default_postgres_user")]
    pub postgres_user: String,
    #[sensitive]
    #[serde(skip_serializing, default = "default_postgres_password")]
    pub postgres_password: String,
    pub postgres_db: String,
    #[sensitive]
    #[serde(skip_serializing)]
    camo_key: Option<String>,
    camo_host: Option<String>,
    static_host: Option<String>,
    #[serde(default = "default_data_root")]
    pub static_root: String,
    pub cdn_host: Option<String>,
    #[serde(default = "default_image_url_root")]
    pub image_url_root: String,
    pub data_root: std::path::PathBuf,
    #[serde(alias = "HTTP_PROXY", alias = "HTTPS_PROXY", alias = "SOCKS_PROXY")]
    pub proxy: Option<url::Url>,
    /// Directory from which to load cryptgraphic keys
    /// You can generate keys by using `tiberius gen-keys ./keys`
    /// Default Value: "./keys"
    #[serde(default = "default_key_directory")]
    pub key_directory: std::path::PathBuf,
    #[serde(alias = "TANTIVY_INDEX", alias = "INDEX_PATH")]
    pub search_dir: std::path::PathBuf,
}

impl Configuration {
    pub async fn db_conn(&self) -> TiberiusResult<DBPool> {
        let opts = sqlx::postgres::PgConnectOptions::new()
            .application_name(&crate::package_full())
            .host(&self.postgres_host)
            .port(self.postgres_port)
            .username(&self.postgres_user)
            .password(&self.postgres_password)
            .database(&self.postgres_db);
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
    pub fn static_host(&self, req: &HTTPReq) -> String {
        self.static_host
            .as_ref()
            .cloned()
            .unwrap_or(req.headers().get_one("host").unwrap_or("localhost").to_string())
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            listen_on: std::net::ToSocketAddrs::to_socket_addrs("localhost:8080")
                .unwrap()
                .next()
                .unwrap(),
            forward_to: std::net::ToSocketAddrs::to_socket_addrs("localhost:8000")
                .unwrap()
                .next()
                .unwrap(),
            session_cookie: default_session_cookie(),
            philomena_encryption_salt: default_philomena_encryption_salt(),
            philomena_signing_salt: default_philomena_signing_salt(),
            postgres_host: "localhost".to_string(),
            postgres_port: 5432,
            postgres_user: "postgres".to_string(),
            postgres_password: "postgres".to_string(),
            postgres_db: "philomena".to_string(),
            camo_host: None,
            camo_key: None,
            static_host: None,
            static_root: "./res".to_string(),
            cdn_host: None,
            image_url_root: "/img".to_string(),
            data_root: PathBuf::from_str("./data").expect("invalid data root path"),
            proxy: None,
            key_directory: PathBuf::from_str("./keys").expect("invalid key dir path"),
            search_dir: PathBuf::from_str("./search").expect("invalid search path")
        }
    }
}
