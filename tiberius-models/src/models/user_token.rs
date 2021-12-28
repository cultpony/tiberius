use std::ops::DerefMut;

use chrono::NaiveDateTime;
use sqlx::query_as;
use tracing::trace;

use crate::{Client, PhilomenaModelError};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct UserToken {
    pub id: i64,
    pub user_id: i64,
    pub token: Vec<u8>,
    pub context: String,
    pub sent_to: Option<String>,
    pub created_at: NaiveDateTime,
}

impl UserToken {
    pub async fn get_user_token_for_session<'a>(
        client: &mut Client,
        user_token: &[u8],
    ) -> Result<Option<UserToken>, PhilomenaModelError> {
        trace!(
            "loading user session for token {}",
            hex::encode(user_token.clone())
        );
        let user_token = query_as!(
            UserToken,
            "SELECT * FROM user_tokens WHERE token = $1 AND context = $2",
            user_token,
            "session"
        )
        .fetch_optional(client.db().await?.deref_mut())
        .await?;
        let user_token = match user_token {
            None => return Ok(None),
            Some(v) => v,
        };
        trace!(
            "session id {} -> user {}",
            user_token.id,
            user_token.user_id
        );
        Ok(Some(user_token))
    }
}
