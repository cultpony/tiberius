use std::ops::DerefMut;

use tiberius_dependencies::chrono::{DateTime, NaiveDate, NaiveDateTime, Utc, Duration};
use ring::rand::SecureRandom;
use sqlx::{query, query_as, types::Uuid};
use std::convert::TryInto;
use tiberius_dependencies::base64;

use crate::{Client, PhilomenaModelError, User};

#[derive(sqlx::FromRow, Debug, Clone, serde::Serialize)]
pub struct ApiKey {
    id: Uuid,
    user_id: i64,
    private: String,
    valid_until: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ApiKey {
    pub fn new(user: &User) -> Result<ApiKey, PhilomenaModelError> {
        let id = Uuid::new_v4();
        let key_data: String = {
            let mut data = [0u8; 64];
            ring::rand::SystemRandom::new().fill(&mut data)?;
            base64::encode(data)
        };
        Ok(ApiKey {
            id,
            user_id: user.id(),
            private: key_data,
            valid_until: Utc::now()
                .checked_add_signed(Duration::weeks(52 * 5))
                .expect("should not overflow"),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
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
            user.id()
        )
        .fetch_all(client)
        .await?;
        Ok(api_keys)
    }
    pub async fn insert(self, client: &mut Client) -> Result<Uuid, PhilomenaModelError> {
        struct UuidW {
            id: Uuid,
        }
        let uuid = query_as!(
            UuidW,
            "INSERT INTO user_api_keys (id, user_id, private, valid_until, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            self.id,
            self.user_id,
            self.private,
            self.valid_until,
            self.created_at,
            self.updated_at,
        ).fetch_one(client).await?;
        Ok(uuid.id)
    }
    pub async fn delete(self, client: &mut Client) -> Result<Uuid, PhilomenaModelError> {
        struct UuidW {
            id: Uuid,
        }
        let id = query_as!(
            UuidW,
            "DELETE FROM user_api_keys WHERE id = $1 RETURNING id",
            self.id,
        )
        .fetch_one(client)
        .await?;
        Ok(id.id)
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
    pub fn user_id(&self) -> i64 {
        self.user_id
    }
}
