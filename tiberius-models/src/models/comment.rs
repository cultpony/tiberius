use async_trait::async_trait;
use sqlx::query_as;
use sqlx::types::ipnetwork::IpNetwork;
use tiberius_dependencies::chrono::NaiveDateTime;

use crate::{Client, Identifiable, IdentifiesUser, PhilomenaModelError, User};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Comment {
    pub id: i32,
    pub body: String,
    pub ip: Option<IpNetwork>,
    pub fingerprint: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub anonymous: Option<bool>,
    pub hidden_from_users: bool,
    pub user_id: Option<i32>,
    pub deleted_by_id: Option<i32>,
    pub image_id: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub edit_reason: Option<String>,
    pub edited_at: Option<NaiveDateTime>,
    pub deletion_reason: String,
    pub destroyed_content: Option<bool>,
    pub name_at_post_time: Option<String>,
}

impl Comment {
    pub async fn author(&self, client: &mut Client) -> Result<Option<User>, PhilomenaModelError> {
        Ok(match self.user_id {
            Some(user_id) => User::get_id(client, user_id as i64).await?,
            None => None,
        })
    }
    pub fn image_id(&self) -> Option<i64> {
        self.image_id.map(|x| x as i64)
    }
    pub async fn get_by_id(
        client: &mut Client,
        id: i64,
    ) -> Result<Option<Comment>, PhilomenaModelError> {
        Ok(
            query_as!(Comment, "SELECT * FROM comments WHERE id = $1", id as i32)
                .fetch_optional(client)
                .await?,
        )
    }
}

impl Identifiable for &Comment {
    fn id(&self) -> i64 {
        self.id as i64
    }
}

#[async_trait]
impl IdentifiesUser for &Comment {
    async fn best_user_identifier(
        &self,
        client: &mut Client,
    ) -> Result<String, PhilomenaModelError> {
        Ok(match self.user_id {
            Some(user_id) => match User::get_id(client, user_id as i64).await? {
                Some(user) => user.id().to_string(),
                None => user_id.to_string(),
            },
            None => match &self.fingerprint {
                Some(fingerprint) => fingerprint.to_string(),
                None => self.created_at.timestamp().to_string(),
            },
        })
    }
    fn user_id(&self) -> Option<i64> {
        self.user_id.clone().map(|x| x as i64)
    }
    fn is_anonymous(&self) -> bool {
        self.anonymous.unwrap_or(false)
    }
}
