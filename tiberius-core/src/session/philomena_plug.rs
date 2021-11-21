use std::convert::TryFrom;

use tiberius_models::{Client, User};

use crate::config::Configuration;
use crate::error::TiberiusResult;
use crate::session::{Session, SessionMode, SessionPtr};

mod session;
#[cfg(test)]
pub(crate) mod session_c;

pub const METADATA_KEY: &str = "_philomena_session_handover";

/// Read the philomena session cookie and update the current session if it is not logged in already
/// If there is a session with an active user, ignore the cookie
pub async fn handover_session(
    client: &mut Client,
    config: &Configuration,
    cookie_value: &str,
    session: SessionPtr<{ SessionMode::Authenticated }>,
) -> TiberiusResult<()> {
    trace!("Attempting session handover");
    let cookie = session::PhilomenaCookie::try_from((config, cookie_value))?;
    let user = User::get_user_for_philomena_token(client, cookie.user_token()).await?;
    trace!("Got user cookie, checking into session");
    if let Some(user) = user {
        let mut session = session.write().await;
        if session.user_id.is_none() {
            trace!("User exists, not logged in, handover accepted");
            session.user_id = Some(user.id());
            session.set_data(METADATA_KEY, "true".into())?;
        } else {
            trace!("User exists but logged in, rejecting handover");
            session.set_data(METADATA_KEY, "rejected".into())?;
        }
    } else {
        trace!("User does not exist, handover failed");
        session
            .write()
            .await
            .set_data(METADATA_KEY, "false".into())?;
    }
    Ok(())
}
