use std::ops::DerefMut;

use chrono::NaiveDateTime;
use maud::{PreEscaped, html};
use sqlx::query_as;

use crate::{Client, PhilomenaModelError, User};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Filter {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub system: bool,
    pub public: bool,
    pub hidden_complex_str: Option<String>,
    pub spoilered_complex_str: Option<String>,
    pub hidden_tag_ids: Vec<i32>,
    pub spoilered_tag_ids: Vec<i32>,
    pub user_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Option<i32>,
}

impl Filter {
    pub async fn default_filter(client: &mut Client) -> Result<Self, PhilomenaModelError> {
        let filter = query_as!(Filter, "SELECT * FROM filters where name = $1", "Default")
            .fetch_one(client)
            .await?;
        Ok(filter)
    }
    pub async fn get_id(client: &mut Client, id: i64) -> Result<Option<Self>, PhilomenaModelError> {
        let filter = query_as!(Filter, "SELECT * FROM filters where id = $1", id as i32)
            .fetch_optional(client)
            .await?;
        Ok(filter)
    }
    pub async fn get_system(client: &mut Client) -> Result<Vec<Filter>, PhilomenaModelError> {
        Ok(query_as!(
            Filter,
            "SELECT * FROM filters WHERE system IS TRUE"
        ).fetch_all(client).await?)
    }
    pub async fn get_user_filters(client: &mut Client, user: &User) -> Result<Vec<Filter>, PhilomenaModelError> {
        Ok(query_as!(
            Filter,
            "SELECT * FROM filters WHERE system IS FALSE AND user_id = $1",
            user.id() as i32
        ).fetch_all(client).await?)
    }
    pub async fn get_user(&self, client: &mut Client) -> Result<Option<User>, PhilomenaModelError> {
        match self.user_id {
            Some(user_id) => User::get_id(client, user_id as i64).await,
            None => Ok(None)
        }
    }
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
