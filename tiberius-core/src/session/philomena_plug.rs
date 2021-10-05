use std::convert::TryFrom;

use tiberius_models::{Client, User};

use crate::config::Configuration;
use crate::error::TiberiusResult;
use crate::session::{Session, SessionPtr};

mod session;
#[cfg(test)]
pub(crate) mod session_c;

/// Read the philomena session cookie and update the current session if it is not logged in already
/// If there is a session with an active user, ignore the cookie
async fn handover_session(client: &mut Client, config: &Configuration, cookie_value: &str, session: SessionPtr) -> TiberiusResult<()> {
    let cookie = session::PhilomenaCookie::try_from((config, cookie_value))?;
    let user = User::get_user_for_philomena_token(client, cookie.user_token()).await?;
    if let Some(user) = user {
        let mut session = session.write().await;
        if session.user_id.is_none() {
            session.user_id = Some(user.id());
        }
    }
    Ok(())
}