


use chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BadgeAward {
    pub id: i32,
    pub label: Option<String>,
    pub awarded_on: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: i32,
    pub badge_id: i32,
    pub awarded_by_id: i32,
    pub reason: Option<String>,
    pub badge_name: Option<String>,
}