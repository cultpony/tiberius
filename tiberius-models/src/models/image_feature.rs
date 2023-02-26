use tiberius_dependencies::chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ImageFeature {
    pub id: i64,
    pub image_id: i64,
    pub user_id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
