use std::ops::DerefMut;

use tiberius_dependencies::chrono::NaiveDateTime;
use sqlx::query_as;
use tracing::trace;

use crate::{Client, PhilomenaModelError};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Forum {
    pub id: i32,
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub access_level: String,
    pub topic_count: i32,
    pub post_count: i32,
    pub watcher_ids: Vec<i32>,
    pub watcher_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_post_id: Option<i32>,
    pub last_topic_id: Option<i32>,
}

impl Forum {
    pub async fn all(client: &mut Client) -> Result<Vec<Self>, PhilomenaModelError> {
        trace!("Getting all forums from database");
        //TODO: load proper permissions from roles
        let forums = query_as!(Forum, "SELECT * FROM forums ORDER BY name")
            .fetch_all(client.db().await?.deref_mut())
            .await?;
        trace!("got {} forums", forums.len());
        Ok(forums)
    }
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
