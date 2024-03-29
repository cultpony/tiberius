use crate::PhilomenaModelError;
use anyhow::Context;
use std::num::NonZeroU32;
use tiberius_dependencies::base64;
use tiberius_dependencies::base64::engine::Engine;
use tiberius_dependencies::totp_rs::{Algorithm, TOTP};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Default)]
pub struct OTPSecret {
    pub encrypted_otp_secret: Option<String>,
    pub encrypted_otp_secret_iv: Option<String>,
    pub encrypted_otp_secret_salt: Option<String>,
    /// The last time we generated a TOTP. This should be updated in the database
    /// after a successfull login, not a failed login!
    pub consumed_timestep: Option<i32>,
    pub otp_required_for_login: Option<bool>,
    pub otp_backup_codes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct OTPEncryptionKey(Vec<u8>);

const fn b64c_default() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        base64::engine::general_purpose::GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true)
            .with_encode_padding(true)
            .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
    )
}

impl OTPSecret {
    pub fn otp_required_for_login(&self) -> bool {
        self.otp_required_for_login.unwrap_or(false)
    }

    pub fn check_otp(
        &self,
        totp: Option<String>,
        key: &OTPEncryptionKey,
    ) -> Result<bool, PhilomenaModelError> {
        let totp: Option<u32> = totp.map(|x| x.trim().parse()).transpose()?;
        let totp_required = self.otp_required_for_login();
        let time = Self::time();
        Ok(match (totp, totp_required) {
            (Some(totp), true) => self
                .algo(key)?
                .map(|algo| algo.check(&totp.to_string(), time))
                .unwrap_or(false),
            (Some(totp), false) => false,
            (None, true) => false,
            (None, false) => true,
        })
    }

    fn algo(&self, key: &OTPEncryptionKey) -> Result<Option<TOTP>, PhilomenaModelError> {
        let dotp = match self.decrypt_otp(&key.0)? {
            Some(d) => d,
            None => return Ok(None),
        };
        // we need the unchecked version since TOTP v4 otherwise throws on shorter secrets that
        // philomena generates
        Ok(Some(TOTP::new_unchecked(Algorithm::SHA1, 6, 1, 30, dotp)))
    }

    fn time() -> u64 {
        let time = tiberius_dependencies::chrono::Utc::now().timestamp();
        assert!(time > 0, "We don't run before 1970");
        time as u64
    }

    pub fn next_otp(&mut self, key: &OTPEncryptionKey) -> Result<Option<u32>, PhilomenaModelError> {
        self.totp_at_timestep(key, Self::time())
    }

    pub fn totp_at_timestep(
        &mut self,
        key: &OTPEncryptionKey,
        time: u64,
    ) -> Result<Option<u32>, PhilomenaModelError> {
        match self.consumed_timestep {
            Some(v) => {
                if v as u64 > time {
                    // already consumed the timestep
                    return Err(PhilomenaModelError::ConsumedTOTPAlready);
                };
                // TODO: update to i64 to prevent 2038 problem
                self.consumed_timestep = Some(time as i32);
            }
            None => self.consumed_timestep = Some(time as i32),
        }
        Ok(self
            .algo(key)?
            .map(|algo| algo.generate(time).parse())
            .transpose()?)
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
        let b64c = b64c_default();
        let secret = self.encrypted_otp_secret.as_ref().unwrap();
        // PG may store garbage codepoints, remove them
        let secret = secret.trim();
        let mut secret = b64c.decode(secret).context("Base64 Secret Decode")?;
        let iv = self.encrypted_otp_secret_iv.as_ref().unwrap();
        // PG may stoer garbage codepoints, remove them
        let iv = iv.trim();
        let iv = b64c.decode(iv).context("Base64 IV Decode")?;
        let iv: Result<[u8; 12], Vec<u8>> = iv.try_into();
        let iv = match iv {
            Ok(v) => v,
            Err(_) => return Err(PhilomenaModelError::Other("Incorrect OTP IV".to_string())),
        };
        let salt = self.encrypted_otp_secret_salt.as_ref().unwrap();
        let salt = salt.trim();
        let salt = salt.trim_start_matches('_');
        let salt = b64c.decode(salt).context("Base64 Salt Decode")?;
        let mut key = [0u8; 32];
        ring::pbkdf2::derive(
            ring::pbkdf2::PBKDF2_HMAC_SHA1,
            NonZeroU32::new(2000).unwrap(),
            &salt,
            otp_secret,
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
            otp_secret,
            &mut key,
        );
        use ring::aead::*;
        let b64c = b64c_default();
        let iv = Nonce::assume_unique_for_key(ivr);
        let key = UnboundKey::new(&ring::aead::AES_256_GCM, &key)?;
        let key = LessSafeKey::new(key);
        let aad = Aad::empty();
        let mut secret = otp.to_vec();
        key.seal_in_place_append_tag(iv, aad, &mut secret)?;
        assert_eq!(secret.len(), otp.len() + 16);
        self.encrypted_otp_secret = Some(b64c.encode(secret));
        self.encrypted_otp_secret_iv = Some(b64c.encode(ivr));
        self.encrypted_otp_secret_salt = Some(b64c.encode(salt));
        Ok(())
    }

    pub fn new_totp_secret(key: &OTPEncryptionKey) -> Result<OTPSecret, PhilomenaModelError> {
        let secret: [u8; 16] = ring::rand::generate(&ring::rand::SystemRandom::new())?.expose();
        let mut s = OTPSecret::default();
        s.encrypt_otp(&key.0, &secret[0..14])?;
        Ok(s)
    }

    pub fn update_totp_secret(
        &mut self,
        key: &OTPEncryptionKey,
    ) -> Result<(), PhilomenaModelError> {
        let secret: [u8; 16] = ring::rand::generate(&ring::rand::SystemRandom::new())?.expose();
        let s = OTPSecret::default();
        self.encrypt_otp(&key.0, &secret[0..14])?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    pub fn test_totp_generation() -> Result<(), PhilomenaModelError> {
        let key = OTPEncryptionKey(
            "xZYTon09JNRrj8snd7KL31wya4x71jmo5aaSSRmw1dGjWLRmEwWMTccwxgsGFGjM"
                .as_bytes()
                .to_vec(),
        );
        let mut s = OTPSecret::new_totp_secret(&key)?;

        // just test that we generate a current token at all
        s.next_otp(&key)?;
        Ok(())
    }

    #[test]
    pub fn test_totp_generation_repeatable() -> Result<(), PhilomenaModelError> {
        let key = OTPEncryptionKey(
            "xZYTon09JNRrj8snd7KL31wya4x71jmo5aaSSRmw1dGjWLRmEwWMTccwxgsGFGjM"
                .as_bytes()
                .to_vec(),
        );
        let mut s = OTPSecret {
            encrypted_otp_secret: Some("m4MihndUmXGTeYWS2eYZlHHIMyZA1m5hAq9NuGXQ".to_string()),
            encrypted_otp_secret_iv: Some("9scsQ4aK37F+6YrR".to_string()),
            encrypted_otp_secret_salt: Some("ve7nL/9eQdKPtKLTwH+ugw==".to_string()),
            consumed_timestep: None,
            otp_required_for_login: None,
            otp_backup_codes: None,
        };

        assert_eq!(
            972311,
            s.totp_at_timestep(&key, 1676616112)?
                .expect("no totp generated despite setup")
        );
        assert_eq!(
            380953,
            s.totp_at_timestep(&key, 1676616192)?
                .expect("no totp generated despite setup")
        );
        assert_eq!(
            596481,
            s.totp_at_timestep(&key, 1676616272)?
                .expect("no totp generated despite setup")
        );
        assert_eq!(
            623007,
            s.totp_at_timestep(&key, 1676616472)?
                .expect("no totp generated despite setup")
        );
        assert_eq!(
            672210,
            s.totp_at_timestep(&key, 1676617192)?
                .expect("no totp generated despite setup")
        );

        assert_eq!(Some(1676617192), s.consumed_timestep);
        Ok(())
    }
}
