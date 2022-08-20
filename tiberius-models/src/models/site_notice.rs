use chrono::NaiveDateTime;
use sqlx::query_as;

use crate::{Client, PhilomenaModelError};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SiteNotice {
    pub id: i32,
    pub title: String,
    pub text: String,
    pub link: String,
    pub link_text: String,
    pub live: bool,
    pub start_date: NaiveDateTime,
    pub finish_date: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: i32,
}

impl SiteNotice {
    pub async fn get_all_active_notices(
        client: &mut Client,
    ) -> Result<Vec<SiteNotice>, PhilomenaModelError> {
        Ok(query_as!(SiteNotice, "SELECT * FROM site_notices WHERE start_date < NOW() AND finish_date > NOW() AND live IS TRUE").fetch_all(client).await?)
    }
}
