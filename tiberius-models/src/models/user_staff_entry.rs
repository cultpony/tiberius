use anyhow::Context;
use tiberius_dependencies::chrono::{DateTime, Utc};
use sqlx::{query, query_as};
use tracing::trace;

use crate::{Client, PhilomenaModelError, User};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct UserStaffEntry {
    pub id: i64,
    pub user_id: i64,
    pub staff_category_id: i64,
    pub display_name: Option<String>,
    pub text: Option<String>,
    pub unavailable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Default for UserStaffEntry {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            user_id: 0,
            staff_category_id: 0,
            display_name: None,
            text: None,
            unavailable: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }
}

impl UserStaffEntry {
    pub async fn get_user(&self, client: &mut Client) -> Result<Option<User>, PhilomenaModelError> {
        Ok(User::get_id(client, self.user_id as i64).await?)
    }

    /// Returns the display name the user configured. It accepts a client to resolve the display name to the user
    /// if necessary (no display name configured) but will not necessarily consume the client
    pub async fn display_name(&self, client: &mut Client) -> Result<String, PhilomenaModelError> {
        Ok(match self.display_name.as_ref() {
            Some(v) => v.clone(),
            None => {
                let user = self.get_user(client).await?;
                match user {
                    None => "???".to_string(),
                    Some(v) => v.displayname().to_string(),
                }
            }
        })
    }

    pub async fn get_for_user(
        user: &User,
        client: &mut Client,
    ) -> Result<Option<Self>, PhilomenaModelError> {
        let r = sqlx::query_as!(
            Self,
            "SELECT * FROM user_staff_entry WHERE user_id = $1 AND deleted_at IS NULL",
            user.id()
        )
        .fetch_optional(client)
        .await?;
        Ok(r)
    }

    pub async fn get_by_id(
        entry_id: i64,
        client: &mut Client,
    ) -> Result<Option<Self>, PhilomenaModelError> {
        let r = sqlx::query_as!(
            Self,
            "SELECT * FROM user_staff_entry WHERE id = $1 AND deleted_at IS NULL",
            entry_id
        )
        .fetch_optional(client)
        .await?;
        Ok(r)
    }

    pub async fn get_all(client: &mut Client) -> Result<Vec<Self>, PhilomenaModelError> {
        let r = sqlx::query_as!(
            Self,
            "SELECT * FROM user_staff_entry WHERE deleted_at IS NULL"
        )
        .fetch_all(client)
        .await?;
        Ok(r)
    }

    pub async fn delete(self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        sqlx::query!("DELETE FROM user_staff_entry WHERE id = $1", self.id)
            .execute(client)
            .await?;
        Ok(())
    }

    /// Saves a new model and updates the ID of the struct to the id of the entry in the database
    /// If the model exists, it is updated
    pub async fn save(&mut self, client: &mut Client) -> Result<(), PhilomenaModelError> {
        let id = sqlx::query!("INSERT INTO
            user_staff_entry (staff_category_id, user_id, display_name, text, created_at, updated_at, deleted_at, unavailable)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id) DO UPDATE
                SET 
                    staff_category_id = excluded.staff_category_id,
                    display_name = excluded.display_name,
                    \"text\" = excluded.text,
                    unavailable = excluded.unavailable,
                    created_at = excluded.created_at,
                    updated_at = excluded.updated_at,
                    deleted_at = excluded.deleted_at
            RETURNING id", self.staff_category_id, self.user_id, self.display_name, self.text, self.created_at, self.updated_at, self.deleted_at, self.unavailable).fetch_one(client)
            .await?;
        self.id = id.id;
        Ok(())
    }
}
