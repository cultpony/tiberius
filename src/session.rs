use anyhow::{Context, Result};
use erlang_term::Term;
use philomena_models::{Client, Filter, User};

#[derive(Clone, securefmt::Debug)]
pub struct Session {
    #[sensitive]
    user: Option<SessionUser>,
}

#[derive(Clone, securefmt::Debug)]
pub struct SessionUser {
    user: philomena_models::User,
    filter: philomena_models::Filter,
    user_token: Vec<u8>,
    totp_token: Option<Vec<u8>>,
    csrf_token: Option<String>,
    live_socket_id: Option<String>,
}

impl Session {
    pub fn has_user(&self) -> bool {
        self.user.is_some()
    }
    pub fn user<'a>(&'a self) -> Option<&'a User> {
        if let Some(user) = &self.user {
            Some(&user.user)
        } else {
            None
        }
    }
    pub fn active_filter<'a>(&'a self) -> Option<&'a Filter> {
        if let Some(user) = &self.user {
            Some(&user.filter)
        } else {
            None
        }
    }
    pub fn csrf_token<'a>(&'a self) -> Option<&'a str> {
        if let Some(user) = &self.user {
            user.csrf_token.as_ref().map(|x| x.as_str())
        } else {
            None
        }
    }
    pub async fn new_from_cookie(client: &mut Client, data: Term) -> Result<Self> {
        let data = data.as_map();
        let data = match data {
            Some(v) => v,
            None => return Ok(Self { user: None }),
        };
        let csrf_token = data.get("_csrf_token").and_then(|x| x.clone().as_string());
        let user_token = data.get("user_token").and_then(|x| x.clone().as_bytes());
        let live_socket_id = data
            .get("live_socket_id")
            .and_then(|x| x.clone().as_string());
        let totp_token = data.get("totp_token").and_then(|x| x.clone().as_bytes());
        match user_token {
            None => return Ok(Self { user: None }),
            Some(user_token) => {
                let user =
                    philomena_models::User::get_user_for_session(client, user_token.clone()).await;
                let user = match user {
                    Ok(v) => v,
                    Err(_) => return Ok(Self { user: None }),
                };
                let user = match user {
                    None => return Ok(Self { user: None }),
                    Some(v) => v,
                };
                Ok(Self {
                    user: Some(SessionUser {
                        filter: user
                            .get_filter(client)
                            .await
                            .context("getting filter for user")?
                            .unwrap_or(Filter::default_filter(client).await?),
                        user,
                        user_token,
                        csrf_token,
                        totp_token,
                        live_socket_id,
                    }),
                })
            }
        }
    }
}
