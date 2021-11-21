use std::ops::DerefMut;

use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use sqlx::{query, query_as};
use tracing::trace;

use crate::{Badge, BadgeAward, Client, Filter, PhilomenaModelError, UserToken};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub encrypted_password: String,
    pub reset_password_token: Option<String>,
    pub reset_password_sent_at: Option<NaiveDateTime>,
    pub remember_created_at: Option<NaiveDateTime>,
    pub sign_in_count: i32,
    pub current_sign_in_at: Option<NaiveDateTime>,
    pub last_sign_in_at: Option<NaiveDateTime>,
    pub current_sign_in_ip: Option<IpNetwork>,
    pub last_sign_in_ip: Option<IpNetwork>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
    pub authentication_token: String,
    pub name: String,
    pub slug: String,
    pub role: String,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub spoiler_type: String,
    pub theme: String,
    pub images_per_page: i32,
    pub show_large_thumbnails: bool,
    pub show_sidebar_and_watched_images: bool,
    pub fancy_tag_field_on_upload: bool,
    pub fancy_tag_field_on_edit: bool,
    pub fancy_tag_field_in_settings: bool,
    pub autorefresh_by_default: bool,
    pub anonymous_by_default: bool,
    pub scale_large_images: bool,
    pub comments_newest_first: bool,
    pub comments_always_jump_to_last: bool,
    pub comments_per_page: i32,
    pub watch_on_reply: bool,
    pub watch_on_new_topic: bool,
    pub watch_on_upload: bool,
    pub messages_newest_first: bool,
    pub serve_webm: bool,
    pub no_spoilered_in_watched: bool,
    pub watched_images_query_str: String,
    pub watched_images_exclude_str: String,
    pub forum_posts_count: i32,
    pub topic_count: i32,
    pub recent_filter_ids: Vec<i32>,
    pub unread_notification_ids: Vec<i32>,
    pub watched_tag_ids: Vec<i32>,
    pub deleted_by_user_id: Option<i32>,
    pub current_filter_id: Option<i32>,
    pub failed_attempts: Option<i32>,
    pub unlock_token: Option<String>,
    pub locked_at: Option<NaiveDateTime>,
    pub uploads_count: i32,
    pub votes_cast_count: i32,
    pub comments_posted_count: i32,
    pub metadata_updates_count: i32,
    pub images_favourited_count: i32,
    pub last_donation_at: Option<NaiveDateTime>,
    pub scratchpad: Option<String>,
    pub use_centered_layout: bool,
    pub secondary_role: Option<String>,
    pub hide_default_role: bool,
    pub personal_title: Option<String>,
    pub show_hidden_items: bool,
    pub hide_vote_counts: bool,
    pub hide_advertisements: bool,
    pub encrypted_otp_secret: Option<String>,
    pub encrypted_otp_secret_iv: Option<String>,
    pub encrypted_otp_secret_salt: Option<String>,
    pub consumed_timestep: Option<i32>,
    pub otp_required_for_login: Option<bool>,
    pub otp_backup_codes: Option<Vec<String>>,
    pub last_renamed_at: NaiveDateTime,
    pub forced_filter_id: Option<i64>,
    pub confirmed_at: Option<NaiveDateTime>,
}

impl User {
    pub fn id(&self) -> i64 {
        self.id as i64
    }
    pub fn displayname(&self) -> &str {
        &self.name
    }
    pub async fn badge_awards(
        &self,
        _client: &mut Client,
    ) -> Result<Vec<BadgeAward>, PhilomenaModelError> {
        todo!("user awards")
    }
    pub async fn badges(&self, _client: &mut Client) -> Result<Vec<Badge>, PhilomenaModelError> {
        todo!("user badges")
    }
    pub async fn get_user_for_session<'a>(
        client: &mut Client,
        session_id: Vec<u8>,
    ) -> Result<Option<User>, PhilomenaModelError> {
        trace!(
            "getting user for session {}",
            hex::encode(session_id.clone())
        );
        let user_token = UserToken::get_user_token_for_session(client, session_id).await?;
        let user_token = match user_token {
            None => return Ok(None),
            Some(v) => v,
        };
        let user = Self::get_id(client, user_token.user_id).await?;
        match &user {
            None => trace!("no user with id {}", user_token.user_id),
            Some(v) => trace!("found user {} for id {}", v.name, user_token.user_id),
        }
        Ok(user)
    }
    pub async fn get_user_for_philomena_token(
        client: &mut Client,
        session_id: &[u8],
    ) -> Result<Option<User>, PhilomenaModelError> {
        let user = query!(
            "SELECT id FROM users WHERE authentication_token = $1",
            base64::encode(session_id),
        )
        .fetch_optional(&mut client.clone())
        .await?;
        if let Some(user) = user {
            let user: i32 = user.id;
            Ok(Self::get_id(client, user as i64).await?)
        } else {
            Ok(None)
        }
    }
    pub async fn get_filter(
        &self,
        client: &mut Client,
    ) -> Result<Option<Filter>, PhilomenaModelError> {
        trace!("getting filter for user {}", self.id);
        Ok(query_as!(
            crate::Filter,
            "SELECT * FROM filters WHERE user_id = $1",
            self.current_filter_id
        )
        .fetch_optional(client.db().await?.deref_mut())
        .await?)
    }
    pub async fn get_id(client: &mut Client, id: i64) -> Result<Option<User>, PhilomenaModelError> {
        Ok(query_as!(crate::User,
            r#"
                SELECT
                    id, email::TEXT as "email!", encrypted_password, reset_password_token, reset_password_sent_at, remember_created_at,
                    sign_in_count, current_sign_in_at, last_sign_in_at, current_sign_in_ip, last_sign_in_ip, created_at, updated_at,
                    deleted_at, authentication_token, name, slug, role, description, avatar, spoiler_type, theme, images_per_page,
                    show_large_thumbnails, show_sidebar_and_watched_images, fancy_tag_field_on_upload, fancy_tag_field_on_edit,
                    fancy_tag_field_in_settings, autorefresh_by_default, anonymous_by_default, scale_large_images, comments_newest_first,
                    comments_always_jump_to_last, comments_per_page, watch_on_reply, watch_on_new_topic, watch_on_upload,
                    messages_newest_first, serve_webm, no_spoilered_in_watched, watched_images_query_str, watched_images_exclude_str,
                    forum_posts_count, topic_count, recent_filter_ids, unread_notification_ids, watched_tag_ids, deleted_by_user_id,
                    current_filter_id, failed_attempts, unlock_token, locked_at, uploads_count, votes_cast_count, comments_posted_count,
                    metadata_updates_count, images_favourited_count, last_donation_at, scratchpad, use_centered_layout,
                    secondary_role, hide_default_role, personal_title, show_hidden_items, hide_vote_counts, hide_advertisements,
                    encrypted_otp_secret, encrypted_otp_secret_iv, encrypted_otp_secret_salt, consumed_timestep, otp_required_for_login,
                    otp_backup_codes, last_renamed_at, forced_filter_id, confirmed_at
                FROM users 
                WHERE 
                    id = $1"#, 
            id as i32
        ).fetch_optional(client.db().await?.deref_mut()).await?)
    }

    pub async fn get_mail_or_name(
        client: &mut Client,
        mail_or_name: String,
    ) -> Result<Option<User>, PhilomenaModelError> {
        let user = query!(
            "SELECT id FROM users WHERE email::TEXT = $1 OR name = $1",
            mail_or_name
        )
        .fetch_optional(client.db().await?.deref_mut())
        .await?;
        if let Some(user) = user {
            let user: i32 = user.id;
            Ok(Self::get_id(client, user as i64).await?)
        } else {
            Ok(None)
        }
    }

    pub async fn get_by_name(
        client: &mut Client,
        name: String,
    ) -> Result<Option<User>, PhilomenaModelError> {
        let user = query!("SELECT id FROM users WHERE name = $1", name)
            .fetch_optional(client.db().await?.deref_mut())
            .await?;
        if let Some(user) = user {
            let user: i32 = user.id;
            Ok(Self::get_id(client, user as i64).await?)
        } else {
            Ok(None)
        }
    }
}
