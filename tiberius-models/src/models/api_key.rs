use std::ops::DerefMut;

use chrono::NaiveDateTime;
use sqlx::{query_as, types::Uuid};
use std::convert::TryInto;

use crate::{Client, PhilomenaModelError, User};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ApiKey {
    id: Uuid,
    user_id: i32,
    private: String,
    valid_until: NaiveDateTime,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl ApiKey {
    pub async fn get_id(
        client: &mut Client,
        id: Uuid,
    ) -> Result<Option<Self>, PhilomenaModelError> {
        let api_key = query_as!(ApiKey, "SELECT * FROM user_api_keys WHERE id = $1", id)
            .fetch_optional(client)
            .await?;
        Ok(api_key)
    }
    pub async fn get_all(
        client: &mut Client,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<ApiKey>, PhilomenaModelError> {
        let offset: i64 = offset.unwrap_or(0).min((i64::MAX - 1).try_into()?) as i64;
        let limit: i64 = limit.unwrap_or(25).min(100) as i64;
        let api_keys = query_as!(
            ApiKey,
            "SELECT * FROM user_api_keys OFFSET $1 LIMIT $2",
            offset,
            limit
        )
        .fetch_all(client)
        .await?;
        Ok(api_keys)
    }
    pub async fn get_all_of_user(
        client: &mut Client,
        user: &User,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<ApiKey>, PhilomenaModelError> {
        let offset: i64 = offset.unwrap_or(0).min((i64::MAX - 1).try_into()?) as i64;
        let limit: i64 = limit.unwrap_or(25).min(100) as i64;
        let api_keys = query_as!(
            ApiKey,
            "SELECT * FROM user_api_keys WHERE user_id = $3 OFFSET $1 LIMIT $2",
            offset,
            limit,
            user.id
        )
        .fetch_all(client)
        .await?;
        Ok(api_keys)
    }
    pub async fn user(&self, client: &mut Client) -> Result<Option<User>, PhilomenaModelError> {
        Ok(User::get_id(client, self.user_id.into()).await?)
    }
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn secret(&self) -> &str {
        &self.private
    }
}
