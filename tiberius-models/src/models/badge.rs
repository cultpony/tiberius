use chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Badge {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub image: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub disable_award: bool,
    pub priority: bool,
}