use std::{collections::BTreeMap, num::NonZeroU32, ops::DerefMut};

use anyhow::Context;
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use either::Either;
use ipnetwork::IpNetwork;
use maud::Markup;
use sqlx::{query, query_as, PgPool};
use tiberius_dependencies::{
    axum_sessions_auth::{Authentication, HasPermission},
    hex, sentry, totp_rs,
};
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

impl Default for User {
    fn default() -> Self {
        let time = Utc::now().naive_utc();
        Self {
            id: 0,
            email: String::default(),
            encrypted_password: String::default(),
            reset_password_token: None,
            reset_password_sent_at: None,
            remember_created_at: None,
            sign_in_count: 0,
            current_sign_in_at: None,
            last_sign_in_at: None,
            current_sign_in_ip: None,
            last_sign_in_ip: None,
            created_at: time,
            updated_at: time,
            deleted_at: None,
            authentication_token: String::default(),
            name: String::default(),
            slug: String::default(),
            role: String::default(),
            description: None,
            avatar: None,
            spoiler_type: String::default(),
            theme: String::default(),
            images_per_page: 20,
            show_large_thumbnails: false,
            show_sidebar_and_watched_images: false,
            fancy_tag_field_on_upload: true,
            fancy_tag_field_on_edit: true,
            fancy_tag_field_in_settings: true,
            autorefresh_by_default: false,
            anonymous_by_default: false,
            scale_large_images: true,
            comments_newest_first: true,
            comments_always_jump_to_last: false,
            comments_per_page: 20,
            watch_on_reply: true,
            watch_on_new_topic: true,
            watch_on_upload: true,
            messages_newest_first: true,
            serve_webm: true,
            no_spoilered_in_watched: true,
            watched_images_query_str: String::default(),
            watched_images_exclude_str: String::default(),
            forum_posts_count: 0,
            topic_count: 0,
            recent_filter_ids: Vec::new(),
            unread_notification_ids: Vec::new(),
            watched_tag_ids: Vec::new(),
            deleted_by_user_id: None,
            current_filter_id: None,
            failed_attempts: None,
            unlock_token: None,
            locked_at: None,
            uploads_count: 0,
            votes_cast_count: 0,
            comments_posted_count: 0,
            metadata_updates_count: 0,
            images_favourited_count: 0,
            last_donation_at: None,
            scratchpad: None,
            use_centered_layout: false,
            secondary_role: None,
            hide_default_role: false,
            personal_title: None,
            show_hidden_items: false,
            hide_vote_counts: false,
            hide_advertisements: false,
            encrypted_otp_secret: None,
            encrypted_otp_secret_iv: None,
            encrypted_otp_secret_salt: None,
            consumed_timestep: None,
            otp_required_for_login: None,
            otp_backup_codes: None,
            last_renamed_at: time,
            forced_filter_id: None,
            confirmed_at: None,
        }
    }
}
pub enum UserLoginResult {
    // Password, User or TOTP invalid
    Invalid,
    // Password valid but TOTP required
    RetryWithTOTP,
    // Password and TOTP correct
    Valid,
}

struct OTPDecrypted {
    secret: Vec<u8>,
    salt: Vec<u8>,
    iv: Vec<u8>,
}

impl User {
    pub fn id(&self) -> i64 {
        self.id as i64
    }
    pub fn displayname(&self) -> &str {
        &self.name
    }
    pub fn avatar(&self) -> Either<&str, Markup> {
        match &self.avatar {
            Some(s) => Either::Left(s.as_str()),
            None => Either::Right(tiberius_common_html::no_avatar_svg()),
        }
    }
    pub fn validate_login<S: Into<String>>(
        &self,
        pepper: Option<&str>,
        otp_secret: &[u8],
        username: &str,
        password: &str,
        totp: Option<S>,
    ) -> Result<UserLoginResult, PhilomenaModelError> {
        let totp: Option<String> = totp.map(|x| x.into());
        if totp.is_none() && self.otp_required_for_login == Some(true) {
            return Ok(UserLoginResult::RetryWithTOTP);
        }
        if username != self.name && username != self.email {
            // Sanity check this but we shouldn't ever hit this code point
            return Ok(UserLoginResult::Invalid);
        }
        let password = format!("{}{}", password, pepper.unwrap_or(""));
        let valid_pw =
            bcrypt::verify(password, &self.encrypted_password).context("BCrypt Verify")?;

        if self.otp_required_for_login.unwrap_or(false) {
            let dotp = self.decrypt_otp(otp_secret).context("TOTP decrypt")?;
            if let Some(totp) = totp {
                if let Some(dotp) = dotp {
                    let dotp = base32::decode(
                        base32::Alphabet::RFC4648 { padding: false },
                        &String::from_utf8_lossy(&dotp),
                    );
                    let dotp = match dotp {
                        None => {
                            return Err(PhilomenaModelError::Context(anyhow::format_err!(
                                "Decode failure on TOTP secret"
                            )))
                        }
                        Some(v) => v,
                    };
                    let time = chrono::Utc::now().timestamp();
                    assert!(time > 0, "We don't run before 1970");
                    let time = time as u64;
                    use totp_rs::{Algorithm, TOTP};
                    let totpi =
                        TOTP::new(Algorithm::SHA1, 6, 1, 30, dotp, None, username.to_string())?;
                    if totpi.check(&totp, time) {
                        return Ok(UserLoginResult::Valid);
                    } else {
                        // TODO: retry with backup codes if not [0-9]{6} format
                        return Ok(UserLoginResult::Invalid);
                    }
                } else {
                    // User has required TOTP but no TOTP setup, so reject the attempt
                    return Ok(UserLoginResult::Invalid);
                }
            } else {
                // User did not supply any TOTP
                return Ok(UserLoginResult::RetryWithTOTP);
            }
        } else {
            return Ok(UserLoginResult::Valid);
        }
    }
    pub(crate) fn decrypt_otp(
        &self,
        otp_secret: &[u8],
    ) -> Result<Option<Vec<u8>>, PhilomenaModelError> {
        if self.encrypted_otp_secret.is_none()
            || self.encrypted_otp_secret_iv.is_none()
            || self.encrypted_otp_secret_salt.is_none()
        {
            return Ok(None);
        }
        trace!(
            "SECRET={:?}, IV={:?}, SALT={:?}",
            self.encrypted_otp_secret,
            self.encrypted_otp_secret_iv,
            self.encrypted_otp_secret_salt
        );
        trace!("OTP_KEY={:?}", otp_secret);
        let b64c = base64::Config::new(base64::CharacterSet::Standard, true)
            .decode_allow_trailing_bits(true);
        let secret = self.encrypted_otp_secret.as_ref().unwrap();
        // PG may store garbage codepoints, remove them
        let secret = secret.trim();
        let mut secret = base64::decode_config(secret, b64c).context("Base64 Secret Decode")?;
        let iv = self.encrypted_otp_secret_iv.as_ref().unwrap();
        // PG may stoer garbage codepoints, remove them
        let iv = iv.trim();
        let iv = base64::decode_config(iv, b64c).context("Base64 IV Decode")?;
        let iv: Result<[u8; 12], Vec<u8>> = iv.try_into();
        let iv = match iv {
            Ok(v) => v,
            Err(_) => return Err(PhilomenaModelError::Other("Incorrect OTP IV".to_string())),
        };
        let salt = self.encrypted_otp_secret_salt.as_ref().unwrap();
        let salt = salt.trim();
        let salt = salt.trim_start_matches('_');
        let salt = base64::decode_config(salt, b64c).context("Base64 Salt Decode")?;
        let mut key = [0u8; 32];
        ring::pbkdf2::derive(
            ring::pbkdf2::PBKDF2_HMAC_SHA1,
            NonZeroU32::new(2000).unwrap(),
            &salt,
            &otp_secret,
            &mut key,
        );
        use ring::aead::*;
        let iv = Nonce::assume_unique_for_key(iv);
        let key = UnboundKey::new(&ring::aead::AES_256_GCM, &key)?;
        let key = LessSafeKey::new(key);
        let aad = Aad::empty();
        let msg = key.open_in_place(iv, aad, &mut secret)?;
        Ok(Some(msg.to_vec()))
    }
    pub(crate) fn encrypt_otp(
        &mut self,
        otp_secret: &[u8],
        otp: &[u8],
    ) -> Result<(), PhilomenaModelError> {
        let salt: [u8; 16] = ring::rand::generate(&ring::rand::SystemRandom::new())?.expose();
        let iv: [u8; 16] = ring::rand::generate(&ring::rand::SystemRandom::new())?.expose();
        let ivr: [u8; 12] = iv[0..12].try_into().unwrap();
        let mut key = [0u8; 32];
        ring::pbkdf2::derive(
            ring::pbkdf2::PBKDF2_HMAC_SHA1,
            NonZeroU32::new(2000).unwrap(),
            &salt,
            &otp_secret,
            &mut key,
        );
        use ring::aead::*;
        let iv = Nonce::assume_unique_for_key(ivr);
        let key = UnboundKey::new(&ring::aead::AES_256_GCM, &key)?;
        let key = LessSafeKey::new(key);
        let aad = Aad::empty();
        let mut secret = otp.to_vec();
        key.seal_in_place_append_tag(iv, aad, &mut secret)?;
        assert_eq!(secret.len(), otp.len() + 16);
        self.encrypted_otp_secret = Some(base64::encode(secret));
        self.encrypted_otp_secret_iv = Some(base64::encode(ivr));
        self.encrypted_otp_secret_salt = Some(base64::encode(salt));
        Ok(())
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
        session_id: &[u8],
    ) -> Result<Option<User>, PhilomenaModelError> {
        trace!(
            "getting user for session {}",
            hex::encode(session_id.clone())
        );
        let user_token = UserToken::get_user_token_for_session(client, &session_id).await?;
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
    pub async fn get_all_user_filters(
        &self,
        client: &mut Client,
    ) -> Result<Vec<Filter>, PhilomenaModelError> {
        Ok(Filter::get_user_filters(client, self).await?)
    }
    pub async fn get_id(client: &mut Client, id: i64) -> Result<Option<User>, PhilomenaModelError> {
        Ok(client.cache_users.get_or_try_insert_with(id, query_as!(crate::User,
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
        ).fetch_optional(&mut client.clone())).await?)
    }

    pub async fn get_mail_or_name(
        client: &mut Client,
        mail_or_name: &str,
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
        name: &str,
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

    pub async fn get_by_email(
        client: &mut Client,
        email: &str,
    ) -> Result<Option<User>, PhilomenaModelError> {
        let user = query!("SELECT id FROM users WHERE email::TEXT = $1", email)
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

impl Into<sentry::User> for User {
    fn into(self) -> sentry::User {
        sentry::User {
            id: Some(self.id.to_string()),
            email: Some(self.email),
            ip_address: self
                .current_sign_in_ip
                .map(|x| sentry::protocol::IpAddress::Exact(x.ip())),
            username: Some(self.name),
            other: BTreeMap::new(),
        }
    }
}

#[async_trait]
impl Authentication<User, PgPool> for User {
    async fn load_user(userid: i64, pool: Option<&PgPool>) -> anyhow::Result<User> {
        let pool = match pool {
            None => anyhow::bail!("no database pool for session layer"),
            Some(pool) => pool.clone(),
        };
        match Self::get_id(&mut pool.into(), userid).await? {
            None => anyhow::bail!("user not found"),
            Some(u) => Ok(u),
        }
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn is_active(&self) -> bool {
        true
    }

    fn is_anonymous(&self) -> bool {
        self.anonymous_by_default
    }
}

#[async_trait]
impl HasPermission<PgPool> for User {
    async fn has(&self, perm: &str, pool: &Option<&PgPool>) -> bool {
        false
    }
}

#[cfg(test)]
mod test {
    use base64::CharacterSet;

    use crate::{PhilomenaModelError, User};

    #[test]
    fn test_encrypt_decrypt_otp() -> Result<(), PhilomenaModelError> {
        let mut user = User::default();
        let otp_secret =
            base64::decode("Wn7O/8DD+qxL0X4X7bvT90wOkVGcA90bIHww4twR03Ci//zq7PnMw8ypqyyT/b/C")
                .unwrap();
        let otp = "AAFFEFASAA1119119DEADBEEF".as_bytes();

        user.encrypt_otp(&otp_secret, otp)
            .expect("could not encrypt OTP secret");

        assert!(user.encrypted_otp_secret.is_some());
        assert!(user.encrypted_otp_secret_iv.is_some());
        assert!(user.encrypted_otp_secret_salt.is_some());

        let r = user
            .decrypt_otp(&otp_secret)
            .expect("could not decrypt OTP secret");
        let r = r.unwrap();

        assert_eq!(otp, r);
        Ok(())
    }

    #[test]
    fn test_philo_decode_otp() -> Result<(), PhilomenaModelError> {
        let b64c =
            base64::Config::new(CharacterSet::Standard, true).decode_allow_trailing_bits(true);
        let test = "VmSaqD2h9SheJO5FXja8dBBV/AvfACBHqjGt+90qAIlJ27V47uGp9A==\x0A        ";
        let test = test.trim();
        let r = base64::decode_config(test, b64c).expect("secret decode failed");
        assert!(r.len() > 0, "Decode must be non-empty");
        Ok(())
    }

    #[test]
    fn test_philo_decode_otp_iv() -> Result<(), PhilomenaModelError> {
        let b64c =
            base64::Config::new(CharacterSet::Standard, true).decode_allow_trailing_bits(true);
        let test = "Jtfmw9tM26CsdyPV\x0A        ";
        let test = test.trim();
        let r = base64::decode_config(test, b64c).expect("IV decode failed");
        assert!(r.len() > 0, "Decode must be non-empty");
        Ok(())
    }

    #[test]
    fn test_philo_decode_otp_salt() -> Result<(), PhilomenaModelError> {
        let b64c =
            base64::Config::new(CharacterSet::Standard, true).decode_allow_trailing_bits(true);
        let test = "_hqD5fUkvYKdA+E77LoDWBA==\x0A        ";
        let test = test.trim();
        let test = test.trim_start_matches('_');
        let r = base64::decode_config(test, b64c).expect("salt decode failed");
        assert!(r.len() > 0, "Decode must be non-empty");
        Ok(())
    }
}
