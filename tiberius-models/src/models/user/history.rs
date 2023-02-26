use tiberius_dependencies::chrono::{Utc, NaiveDateTime};
use sqlx::types::ipnetwork::IpNetwork;

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct UserHistory {
    pub remember_created_at: Option<NaiveDateTime>,
    pub sign_in_count: i32,
    pub current_sign_in_at: Option<NaiveDateTime>,
    pub last_sign_in_at: Option<NaiveDateTime>,
    pub current_sign_in_ip: Option<IpNetwork>,
    pub last_sign_in_ip: Option<IpNetwork>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
    pub deleted_by_user_id: Option<i32>,

    pub locked_at: Option<NaiveDateTime>,
    pub uploads_count: i32,
    pub votes_cast_count: i32,
    pub comments_posted_count: i32,
    pub metadata_updates_count: i32,
    pub images_favourited_count: i32,
    pub last_donation_at: Option<NaiveDateTime>,
    pub forum_posts_count: i32,
    pub topic_count: i32,

    pub last_renamed_at: NaiveDateTime,
    pub confirmed_at: Option<NaiveDateTime>,
    pub failed_attempts: Option<i32>,

}

impl Default for UserHistory {
    fn default() -> Self {
        let time = Utc::now().naive_utc();
        Self {
            remember_created_at: None,
            sign_in_count: 0,
            current_sign_in_at: None,
            last_sign_in_at: None,
            current_sign_in_ip: None,
            last_sign_in_ip: None,

            created_at: time,
            updated_at: time,
            deleted_at: None,
            deleted_by_user_id: None,

            locked_at: None,
            uploads_count: 0,
            votes_cast_count: 0,
            comments_posted_count: 0,
            metadata_updates_count: 0,
            images_favourited_count: 0,
            last_donation_at: None,
            forum_posts_count: 0,
            topic_count: 0,

            last_renamed_at: time,
            confirmed_at: None,
            failed_attempts: None,
        }
    }
}