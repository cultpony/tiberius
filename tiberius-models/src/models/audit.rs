use sqlx::query_as;

use crate::{Client, Image, PhilomenaModelError, User};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct AuditImage {
    id: i64,
    image_id: i64,
    user_id: i64,
    change: serde_json::Value,
    reason: String,
}

impl AuditImage {
    pub async fn new(
        client: &mut Client,
        image: &Image,
        user: &User,
        change: serde_json::Value,
        reason: String,
    ) -> Result<i64, PhilomenaModelError> {
        #[derive(sqlx::FromRow)]
        struct Returning {
            id: i64,
        }
        let id = query_as!(
            Returning,
            "INSERT INTO audit_images (image_id, user_id, change, reason) VALUES ($1, $2, $3, $4)
            RETURNING id",
            image.id as i64,
            user.id as i64,
            change,
            reason,
        )
        .fetch_one(client)
        .await?;
        Ok(id.id)
    }

    pub async fn fetch(
        client: &mut Client,
        id: i64,
    ) -> Result<Option<AuditImage>, PhilomenaModelError> {
        let audit_record = query_as!(AuditImage, "SELECT * FROM audit_images WHERE id = $1", id,)
            .fetch_optional(client)
            .await?;
        Ok(audit_record)
    }
}
