use std::convert::TryFrom;

use anyhow::Context;
use tiberius_models::{Client, User, UserToken};

use crate::config::Configuration;
use crate::error::TiberiusResult;
use crate::session::{Authenticated, Session, SessionMode, SessionPtr};

mod session;
#[cfg(test)]
pub(crate) mod session_c;

pub const METADATA_KEY: &str = "_philomena_session_handover";

/// Read the philomena session cookie and update the current session if it is not logged in already
/// If there is a session with an active user, ignore the cookie
pub async fn handover_session<T: SessionMode>(
    client: &mut Client,
    config: &Configuration,
    cookie_value: &str,
    session: SessionPtr<T>,
) -> TiberiusResult<()> {
    trace!("Attempting session handover");
    let cookie = session::PhilomenaCookie::try_from((config, cookie_value))?;
    let user_token = cookie.user_token();
    match user_token {
        None => {
            trace!("No user token, user logged out, terminating session");
            let mut session = session.write().await;
            // Turn session into unauthenticated
            session.set_data(METADATA_KEY, "terminated".into())?;
            session.user_id = None;
            Ok(())
        }
        Some(user_token) => {
            let user = User::get_user_for_session(client, user_token).await?;
            // double check existing METADATA is not overwritten to preserve status of session handover
            trace!("Got user cookie, checking into session");
            let session_status = session
                .read()
                .await
                .get_data(METADATA_KEY)
                .context("session read failure for handover")?;
            match session_status.unwrap_or_default().as_str() {
                // We might want to retry handover in some cases, so this is TODO:
                "false" => (),
                "rejected" => {
                    if session.read().await.user_id.is_some() {
                        return Ok(());
                    }
                }
                "true" => return Ok(()),
                // User logged out, revalidate!
                "terminated" => (),
                "" => (),
                _ => unreachable!(),
            }
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
                trace!(
                    "User for token {:?} does not exist, handover failed",
                    hex::encode(user_token)
                );
                session
                    .write()
                    .await
                    .set_data(METADATA_KEY, "false".into())?;
            }
            Ok(())
        }
    }
}
