use std::ops::DerefMut;

use chrono::NaiveDateTime;
use sqlx::{query_as, };

use crate::{Client, PhilomenaModelError};

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
            .fetch_one(client.db().await?.deref_mut())
            .await?;
        Ok(filter)
    }
    pub async fn get_id(client: &mut Client, id: i64) -> Result<Option<Self>, PhilomenaModelError> {
        let filter = query_as!(Filter, "SELECT * FROM filters where id = $1", id as i32)
            .fetch_optional(client.db().await?.deref_mut())
            .await?;
        Ok(filter)
    }
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
