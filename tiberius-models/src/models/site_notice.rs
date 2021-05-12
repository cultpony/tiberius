use chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SiteNotice {
    pub id: i32,
    pub title: String,
    pub text: String,
    pub link: Option<String>,
    pub link_text: Option<String>,
    pub live: bool,
    pub start_date: NaiveDateTime,
    pub finish_date: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: i32,
}
