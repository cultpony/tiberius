use std::{collections::BTreeMap, num::NonZeroU32, ops::DerefMut};

use anyhow::Context;
use async_trait::async_trait;
use either::Either;
use maud::Markup;
use sqlx::Executor;
use sqlx::{query, query_as, types::ipnetwork::IpNetwork, PgPool};
use tiberius_dependencies::base32;
use tiberius_dependencies::base64;
use tiberius_dependencies::chrono::{NaiveDateTime, Utc};
use tiberius_dependencies::{
    axum_sessions_auth::{Authentication, HasPermission},
    hex, sentry, totp_rs,
};
use tracing::trace;

pub mod history;
pub mod otp;
pub mod settings;
pub use history::UserHistory;
pub use otp::OTPSecret;
pub use settings::UserSettings;

use crate::{Badge, BadgeAward, Client, Filter, PhilomenaModelError, UserToken};

#[derive(
    sqlx::Type, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Default,
)]
#[repr(transparent)]
#[sqlx(rename = "citext")]
pub struct CiText(String);

impl From<CiText> for String {
    fn from(value: CiText) -> Self {
        value.0.to_lowercase()
    }
}

impl From<String> for CiText {
    fn from(f: String) -> Self {
        Self(f.to_lowercase())
    }
}

impl AsRef<str> for CiText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CiText {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0.to_lowercase())
    }
}

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct User {
    pub id: i32,
    pub email: CiText,
    pub encrypted_password: String,
    pub reset_password_token: Option<String>,
    pub reset_password_sent_at: Option<NaiveDateTime>,
    pub authentication_token: String,
    pub name: String,
    pub slug: String,
    pub role: String,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub unread_notification_ids: Vec<i32>,
    pub unlock_token: Option<String>,
    pub scratchpad: Option<String>,
    pub secondary_role: Option<String>,
    pub personal_title: Option<String>,
    #[sqlx(flatten)]
    pub otp_secret: OTPSecret,
    #[sqlx(flatten)]
    pub user_history: UserHistory,
    #[sqlx(flatten)]
    pub user_settings: UserSettings,
}
impl Default for User {
    fn default() -> Self {
        let time = Utc::now().naive_utc();
        Self {
            id: 0,
            email: String::default().into(),
            encrypted_password: String::default(),
            reset_password_token: None,
            reset_password_sent_at: None,
            authentication_token: String::default(),
            name: String::default(),
            slug: String::default(),
            role: String::default(),
            description: None,
            avatar: None,
            unread_notification_ids: Vec::new(),
            unlock_token: None,
            scratchpad: None,
            secondary_role: None,
            personal_title: None,
            otp_secret: OTPSecret::default(),
            user_history: UserHistory::default(),
            user_settings: UserSettings::default(),
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
        if totp.is_none() && self.otp_secret.otp_required_for_login == Some(true) {
            return Ok(UserLoginResult::RetryWithTOTP);
        }
        if username != self.name && username != self.email.as_ref() {
            // Sanity check this but we shouldn't ever hit this code point
            return Ok(UserLoginResult::Invalid);
        }
        let password = format!("{}{}", password, pepper.unwrap_or(""));
        let valid_pw =
            bcrypt::verify(password, &self.encrypted_password).context("BCrypt Verify")?;

        if self.otp_secret.otp_required_for_login() {
            let dotp = self
                .otp_secret
                .decrypt_otp(otp_secret)
                .context("TOTP decrypt")?;
            if let Some(totp) = totp {
                if let Some(dotp) = dotp {
                    //debug!("TOTP secret = {:?}", String::from_utf8_lossy(&dotp));
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
                    //assert!(dotp.len() >= 120 / 8, "TOTP Secret Insufficient Size");
                    let time = tiberius_dependencies::chrono::Utc::now().timestamp();
                    assert!(time > 0, "We don't run before 1970");
                    let time = time as u64;
                    use totp_rs::{Algorithm, TOTP};
                    let totpi = TOTP::new_unchecked(Algorithm::SHA1, 6, 1, 30, dotp);
                    if totpi.check(&totp, time) {
                        Ok(UserLoginResult::Valid)
                    } else {
                        debug!("Invalid TOTP, stopping login session");
                        // TODO: retry with backup codes if not [0-9]{6} format
                        Ok(UserLoginResult::Invalid)
                    }
                } else {
                    // User has required TOTP but no TOTP setup, so reject the attempt
                    Ok(UserLoginResult::Invalid)
                }
            } else {
                // User did not supply any TOTP
                Ok(UserLoginResult::RetryWithTOTP)
            }
        } else {
            Ok(UserLoginResult::Valid)
        }
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
        trace!("getting user for session {}", hex::encode(session_id));
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
            self.user_settings.current_filter_id
        )
        .fetch_optional(client.db().await?.deref_mut())
        .await?)
    }

    pub async fn get_all_user_filters(
        &self,
        client: &mut Client,
    ) -> Result<Vec<Filter>, PhilomenaModelError> {
        Filter::get_user_filters(client, self).await
    }

    #[cfg(test)]
    pub async fn new_test_user(client: &mut Client) -> Result<Self, PhilomenaModelError> {
        let user = User {
            id: 0x5EADBEEFi32,
            email: "testuser@email.com".to_string().into(),
            name: "testuser".to_string(),
            slug: "testuser".to_string(),
            ..Default::default()
        };
        const QUERY: &str = r#"
        INSERT INTO users (
            id, email, name, slug, created_at, updated_at,
            authentication_token, role
        )
        VALUES ($1, $2, $3, $4, $5, $6, '', '');
        "#;
        let query = sqlx::query(QUERY)
            .bind(user.id)
            .bind(user.email)
            .bind(user.name)
            .bind(user.slug)
            .bind(user.user_history.created_at)
            .bind(user.user_history.updated_at);
        client.execute(query).await?;
        let user = User::get_id(client, user.id.into())
            .await?
            .expect("just created user, did not read back");
        Ok(user)
    }

    pub async fn get_id(client: &mut Client, id: i64) -> Result<Option<User>, PhilomenaModelError> {
        const QUERY: &str = r#"
        SELECT
            *
        FROM users 
        WHERE 
            id = $1"#;
        use futures::FutureExt;
        let query = sqlx::query(QUERY).bind(id);
        let fetch = client
            .fetch_optional(query)
            .map(|f| -> Result<Option<User>, sqlx::Error> {
                f.map(|f| {
                    use sqlx::FromRow;
                    f.map(|f| User::from_row(&f))
                })?
                .transpose()
            });
        Ok(client.cache_users.try_get_with(id, fetch).await?)
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

impl From<User> for sentry::User {
    fn from(value: User) -> Self {
        sentry::User {
            id: Some(value.id.to_string()),
            email: Some(value.email.into()),
            ip_address: value
                .user_history
                .current_sign_in_ip
                .map(|x| sentry::protocol::IpAddress::Exact(x.ip())),
            username: Some(value.name),
            other: BTreeMap::new(),
        }
    }
}

#[async_trait]
impl Authentication<User, i64, PgPool> for User {
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
        self.user_settings.anonymous_by_default
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

    use super::*;
    use crate::{Client, PhilomenaModelError, User};
    use tiberius_dependencies::base64;
    use tiberius_dependencies::base64::Engine;

    #[test]
    fn test_encrypt_decrypt_otp() -> Result<(), PhilomenaModelError> {
        let mut user = User::default();
        let otp_secret =
            base64::decode("Wn7O/8DD+qxL0X4X7bvT90wOkVGcA90bIHww4twR03Ci//zq7PnMw8ypqyyT/b/C")
                .unwrap();
        let otp = "AAFFEFASAA1119119DEADBEEF".as_bytes();

        user.otp_secret
            .encrypt_otp(&otp_secret, otp)
            .expect("could not encrypt OTP secret");

        assert!(user.otp_secret.encrypted_otp_secret.is_some());
        assert!(user.otp_secret.encrypted_otp_secret_iv.is_some());
        assert!(user.otp_secret.encrypted_otp_secret_salt.is_some());

        let r = user
            .otp_secret
            .decrypt_otp(&otp_secret)
            .expect("could not decrypt OTP secret");
        let r = r.unwrap();

        assert_eq!(otp, r);
        Ok(())
    }

    #[test]
    fn test_philo_decode_otp() -> Result<(), PhilomenaModelError> {
        let b64c = base64::engine::general_purpose::GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true);
        let b64c =
            base64::engine::general_purpose::GeneralPurpose::new(&base64::alphabet::STANDARD, b64c);
        let test = "VmSaqD2h9SheJO5FXja8dBBV/AvfACBHqjGt+90qAIlJ27V47uGp9A==\x0A        ";
        let test = test.trim();
        let r = b64c.decode(test).expect("secret decode failed");
        assert!(!r.is_empty(), "Decode must be non-empty");
        Ok(())
    }

    #[test]
    fn test_philo_decode_otp_iv() -> Result<(), PhilomenaModelError> {
        let b64c = base64::engine::general_purpose::GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true);
        let b64c =
            base64::engine::general_purpose::GeneralPurpose::new(&base64::alphabet::STANDARD, b64c);
        let test = "Jtfmw9tM26CsdyPV\x0A        ";
        let test = test.trim();
        let r = b64c.decode(test).expect("IV decode failed");
        assert!(!r.is_empty(), "Decode must be non-empty");
        Ok(())
    }

    #[test]
    fn test_philo_decode_otp_salt() -> Result<(), PhilomenaModelError> {
        let b64c = base64::engine::general_purpose::GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true);
        let b64c =
            base64::engine::general_purpose::GeneralPurpose::new(&base64::alphabet::STANDARD, b64c);
        let test = "_hqD5fUkvYKdA+E77LoDWBA==\x0A        ";
        let test = test.trim();
        let test = test.trim_start_matches('_');
        let r = b64c.decode(test).expect("salt decode failed");
        assert!(!r.is_empty(), "Decode must be non-empty");
        Ok(())
    }

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "../migrations"))]
    async fn test_user_create_and_fetch() -> Result<(), PhilomenaModelError> {
        let mut client = Client::new(pool, None);
        let user = User::new_test_user(&mut client).await?;

        let user2 = User::get_id(&mut client, user.id.into()).await?;
        assert_eq!(Some(user), user2);
        Ok(())
    }
}
