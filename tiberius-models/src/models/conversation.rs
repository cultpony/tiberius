use chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Conversation {
    pub id: i32,
    pub title: String,
    pub to_read: bool,
    pub from_read: bool,
    pub to_hidden: bool,
    pub from_hidden: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub from_id: i32,
    pub to_id: i32,
    pub slug: String,
    pub last_message_at: NaiveDateTime,
}
