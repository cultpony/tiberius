use tiberius_dependencies::chrono::NaiveDateTime;

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

impl Badge {
    pub fn title(&self) -> String {
        format!("{} - {}", self.title, self.description)
    }
}
