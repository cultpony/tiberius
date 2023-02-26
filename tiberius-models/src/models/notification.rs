use tiberius_dependencies::chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Notification {
    pub id: i32,
    pub action: String,
    pub watcher_ids: Vec<i32>,
    pub actor_id: i32,
    pub actor_type: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub actor_child_id: Option<i32>,
    pub actor_child_type: Option<String>,
}
