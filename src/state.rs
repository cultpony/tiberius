use crate::{
    config::Configuration, request_helper::SafeSqlxRequestExt, session::Session, StatelessPaths,
};
use anyhow::Result;
use log::{trace, warn};
use philomena_models::ApiKey;
use tide::Request;

#[derive(Clone)]
pub struct State {
    pub config: Configuration,
    pub keydata: crate::app::cookie_check::KeyData,
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
        let keydata = crate::app::cookie_check::KeyData::new_str(
            &config.philomena_secret,
            &config.philomena_encryption_salt,
            &config.philomena_signing_salt,
        )?;
        Ok(Self { config, keydata })
    }
    pub fn config(&self) -> &Configuration {
        &self.config
    }
}

pub struct StateMiddleware;

#[tide::utils::async_trait]
impl tide::Middleware<State> for StateMiddleware {
    async fn handle(&self, mut req: Request<State>, next: tide::Next<'_, State>) -> tide::Result {
        let req = if StatelessPaths::contains(req.url().path()) {
            trace!("skipping state for stateless path {:?}", req.url().path());
            req
        } else {
            trace!("applying state for statefull path {:?}", req.url().path());
            state_middleware(req).await
        };
        let res = next.run(req).await;
        Ok(res)
    }
}

pub async fn state_middleware(mut req: Request<State>) -> Request<State> {
    let cookie = req.cookie(&req.state().config.session_cookie);
    let cookie = match cookie {
        None => return req,
        Some(cookie) => cookie,
    };
    let data: Result<Vec<u8>> = req
        .state()
        .keydata
        .decrypt_and_verify_cookie(cookie.value().as_bytes());
    match data {
        Err(e) => {
            trace!("couldn't decode cookie: {}", e);
            req
        }
        Ok(data) => {
            let term = erlang_term::Term::from_bytes(&data);
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
            }
        }
    }
}
