use std::sync::Arc;

use crate::{
    app::DBPool,
    config::Configuration,
    error::{TiberiusError, TiberiusResult},
    pages::error_page,
    request_helper::SafeSqlxRequestExt,
    StatelessPaths,
};
use async_std::path::Path;
use chrono::Utc;
use philomena_models::{ApiKey, Client, Filter, User};
use rocket::{fairing::Fairing, Request};

#[derive(Clone)]
pub struct State {
    pub config: Configuration,
    pub cryptokeys: CryptoKeys,
    pub db_pool: DBPool,
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

impl State {
    pub async fn new(config: Configuration) -> TiberiusResult<Self> {
        let cryptokeys = {
            log::info!("Loading cryptographic keys");
            let path = config.key_directory.canonicalize()?;
            let ed25519key = async_std::fs::read(path.join(Path::new("ed25519.pkcs8"))).await?;
            let randomkeystr = async_std::fs::read(path.join(Path::new("main.key"))).await?;
            assert!(randomkeystr.len() == 64, "Random key must have 64 bytes");
            let ed25519key = ring::signature::Ed25519KeyPair::from_pkcs8(&ed25519key)?;
            let mut randomkey: [u8; 64] = [0; 64];
            for char in 0..64 {
                randomkey[char] = randomkeystr[char];
            }
            CryptoKeys {
                signing_key: Arc::new(ed25519key),
                random_key: randomkey,
            }
        };
        let db_pool = config.db_conn().await?;
        Ok(Self {
            config,
            cryptokeys,
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
}
