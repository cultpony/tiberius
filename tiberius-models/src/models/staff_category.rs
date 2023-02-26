use std::{num::NonZeroU32, ops::DerefMut};

use anyhow::Context;
use tiberius_dependencies::chrono::{DateTime, Utc};
use sqlx::{query, query_as};
use tracing::trace;

use crate::{Client, PhilomenaModelError, User};

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[repr(i32)]
pub enum StaffCategoryColor {
    #[serde(rename = "none")]
    None = 0,
    #[serde(rename = "red")]
    Red = 1,
    #[serde(rename = "orange")]
    Orange = 2,
    #[serde(rename = "green")]
    Green = 3,
    #[serde(rename = "purple")]
    Purple = 4,
}

impl ToString for StaffCategoryColor {
    fn to_string(&self) -> String {
        use StaffCategoryColor::*;
        match self {
            Red => "block--danger",
            Orange => "block--warning",
            Green => "block--success",
            Purple => "block--assistant",
            None => "",
        }
        .to_string()
    }
}

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct StaffCategory {
    pub id: i64,
    pub role: String,
    pub ordering: i64,
    pub color: StaffCategoryColor,
    pub display_name: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Default for StaffCategory {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            role: String::default(),
            ordering: 0,
            color: StaffCategoryColor::None,
            display_name: String::default(),
            text: String::default(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }
}

impl StaffCategory {
    pub async fn get_all(client: &mut Client) -> Result<Vec<Self>, PhilomenaModelError> {
        let r = sqlx::query_as!(Self, r#"SELECT id, role, ordering, color as "color: StaffCategoryColor", display_name, text, created_at, updated_at, deleted_at FROM staff_category WHERE deleted_at IS NULL ORDER BY ordering, id"#)
            .fetch_all(client)
            .await?;
        Ok(r)
    }

    pub async fn delete(self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        sqlx::query!("DELETE FROM staff_category WHERE id = $1", self.id)
            .execute(client)
            .await?;
        Ok(())
    }

    pub async fn save(&mut self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        let id = sqlx::query!(
            "INSERT INTO
            staff_category (role, display_name, text, created_at, updated_at, deleted_at, color)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (role) DO UPDATE
                SET 
                    display_name = excluded.display_name,
                    \"role\" = excluded.role,
                    created_at = excluded.created_at,
                    updated_at = excluded.updated_at,
                    deleted_at = excluded.deleted_at,
                    color = excluded.color
            RETURNING id",
            self.role,
            self.display_name,
            self.text,
            self.created_at,
            self.updated_at,
            self.deleted_at,
            self.color as i32
        )
        .fetch_one(client)
        .await?;
        self.id = id.id;
        Ok(())
    }

    pub fn category(&self) -> StaffCategoryColor {
        self.color
    }
}
