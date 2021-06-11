use std::sync::Arc;

use crate::{
    config::Configuration, pages::error_page, request_helper::SafeSqlxRequestExt, StatelessPaths,
};
use anyhow::Result;
use async_std::path::Path;
use chrono::Utc;
use log::{trace, warn};
use openssl::rand;
use philomena_models::{ApiKey, Filter, User};
use tide::Request;
use tokio::sync::OnceCell;

#[derive(Clone)]
pub struct State {
    pub config: Configuration,
    pub keydata: Option<crate::app::cookie_check::KeyData>,
    pub cryptokeys: CryptoKeys,
}

#[derive(Clone)]
pub struct CryptoKeys {
    pub signing_key: Arc<ring::signature::Ed25519KeyPair>,
    pub random_key: [u8; 64],
}

pub struct Footer {
    pub aud: String,
    pub exp: chrono::DateTime<Utc>,
    pub iss: chrono::DateTime<Utc>,
}

#[derive(Clone)]
pub struct AuthToken(pub String);

impl Into<ApiKey> for AuthToken {
    fn into(self) -> ApiKey {
        ApiKey::new(Some(self.0))
    }
}

impl State {
    pub async fn new(config: Configuration) -> Result<Self> {
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
        if let Some(philomena_secret) = &config.philomena_secret {
            let keydata = crate::app::cookie_check::KeyData::new_str(
                &philomena_secret,
                &config.philomena_encryption_salt,
                &config.philomena_signing_salt,
            )?;
            Ok(Self {
                config,
                keydata: Some(keydata),
                cryptokeys,
            })
        } else {
            Ok(Self {
                config,
                keydata: None,
                cryptokeys,
            })
        }
    }
    pub fn config(&self) -> &Configuration {
        &self.config
    }

    /// The data will be signed and encrypted securely
    /// The resulting string can be freely sent to the user without being able to inspect the data itself
    pub fn encrypt_data<T: serde::ser::Serialize>(&self, ft: Footer, data: &T) -> Result<String> {
        let msg = serde_cbor::to_vec(data)?;
        let msg = base64::encode(msg);
        let footer = "";
        match paseto::v2::local_paseto(&msg, Some(footer), &self.cryptokeys.random_key) {
            Ok(v) => Ok(v),
            Err(e) => anyhow::bail!("error in paseto: {}", e),
        }
    }
    pub fn decrypt_data<T: serde::de::DeserializeOwned, S: Into<String>>(&self, data: S) -> T {
        let data: String = data.into();
        todo!()
    }
}

pub struct StateMiddleware;

#[tide::utils::async_trait]
impl tide::Middleware<State> for StateMiddleware {
    async fn handle(&self, req: Request<State>, next: tide::Next<'_, State>) -> tide::Result {
        let req = if StatelessPaths::contains(req.url().path()) {
            trace!("skipping state for stateless path {:?}", req.url().path());
            req
        } else {
            trace!("applying state for statefull path {:?}", req.url().path());
            state_middleware(req).await?
        };
        let res = next.run(req).await;
        Ok(res)
    }
}

pub async fn state_middleware(mut req: Request<State>) -> Result<Request<State>> {
    let mut client = req.get_db_client().await?;

    let session = req.session_mut();

    let user_id: Option<i64> = session.get::<i64>("user");
    let default_filter = Filter::default_filter(&mut client).await?;
    if let Some(user_id) = user_id {
        let user = User::get_id(&mut client, user_id).await?;

        if let Some(user) = user {
            // Logged in user sessions last for 6 days if they're not seen
            // TODO: use remember_me token instead to revive session
            session.expire_in(std::time::Duration::from_secs(6 * 24 * 60 * 60));
            req.set_ext(
                user.get_filter(&mut client)
                    .await?
                    .unwrap_or(default_filter),
            );
            req.set_ext(AuthToken(user.authentication_token.clone()));
        }

        Ok(req)
    } else {
        // TODO: handle logged out view
        if let Some(filter_id) = session.get::<i64>("filter") {
            req.set_ext(
                Filter::get_id(&mut client, filter_id)
                    .await?
                    .unwrap_or(default_filter),
            );
        }
        Ok(req)
    }
    /*let cookie = req.cookie(&req.state().config.session_cookie);
    let cookie = match cookie {
        None => return req,
        Some(cookie) => cookie,
    };
    let data: Result<Option<Vec<u8>>> = req
        .state()
        .keydata
        .as_ref()
        .map(|x| x.decrypt_and_verify_cookie(cookie.value().as_bytes()))
        .transpose();
    match data {
        Err(e) => {
            trace!("couldn't decode cookie: {}", e);
            req
        },
        Ok(None) => {
            req
        },
        Ok(Some(data)) => {
            let session = req.session_mut();
            todo!()
            /*let term = erlang_term::Term::from_bytes(&data);
            match term {
                Ok(term) => {
                    let client = req.get_db_client().await;
                    let mut client = match client {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("could not get database from session");
                            return req;
                        }
                    };
                    let session: Result<Session> =
                        Session::new_from_cookie(&mut client, term).await;
                    drop(client);
                    let session = match session {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("could not find session for user: {}", e);
                            return req;
                        }
                    };
                    if let Some(active_filter) = session.active_filter() {
                        req.set_ext(active_filter.clone());
                    }
                    if let Some(user) = session.user() {
                        req.set_ext(user.clone());
                        req.set_ext(AuthToken(user.authentication_token.clone()));
                    }
                    req.set_ext(session);
                    req
                }
                Err(e) => {
                    trace!("invalid term: {}", e);
                    req
                }
            }*/
        }
    }*/
}
