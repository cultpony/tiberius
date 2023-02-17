use crate::PhilomenaModelError;
use anyhow::Context;
use std::num::NonZeroU32;
use tiberius_dependencies::totp_rs::{Algorithm,TOTP};

#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct OTPSecret {
    pub encrypted_otp_secret: Option<String>,
    pub encrypted_otp_secret_iv: Option<String>,
    pub encrypted_otp_secret_salt: Option<String>,
    pub consumed_timestep: Option<i32>,
    pub otp_required_for_login: Option<bool>,
    pub otp_backup_codes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct OTPEncryptionKey(Vec<u8>);

impl Default for OTPSecret {
    fn default() -> Self {
        Self {
            encrypted_otp_secret: None,
            encrypted_otp_secret_iv: None,
            encrypted_otp_secret_salt: None,
            consumed_timestep: None,
            otp_required_for_login: None,
            otp_backup_codes: None,
        }
    }
}

impl OTPSecret {
    pub fn otp_required_for_login(&self) -> bool {
        self.otp_required_for_login.unwrap_or(false)
    }

    pub fn check_otp(&self, totp: Option<String>, key: &OTPEncryptionKey) -> Result<bool, PhilomenaModelError> {
        let totp: Option<u32> = totp.map(|x| x.trim().parse()).transpose()?;
        let totp_required = self.otp_required_for_login();
        let time = Self::time();
        Ok(match (totp, totp_required) {
            (Some(totp), true) => self.algo(&key)?.map(|algo| algo.check(&*totp.to_string(), time)).unwrap_or(false),
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
        Ok(Some(TOTP::new(Algorithm::SHA1, 6, 1, 30, dotp, None, "".to_string())?))
    }

    fn time() -> u64 {
        let time = chrono::Utc::now().timestamp();
        assert!(time > 0, "We don't run before 1970");
        time as u64
    }

    pub fn next_otp(&self, key: &OTPEncryptionKey) -> Result<Option<u32>, PhilomenaModelError> {
        Ok(self.algo(key)?.map(|algo| algo.generate(Self::time()).parse()).transpose()?)
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

    pub fn new_totp_secret(key: &OTPEncryptionKey) -> Result<OTPSecret, PhilomenaModelError> {
        let secret: [u8; 16] = ring::rand::generate(&ring::rand::SystemRandom::new())?.expose();
        let mut s = OTPSecret::default();
        s.encrypt_otp(&key.0, &secret[0..14])?;
        Ok(s)
    }

    pub fn update_totp_secret(&mut self, key: &OTPEncryptionKey) -> Result<(), PhilomenaModelError> {
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
        let key = OTPEncryptionKey("xZYTon09JNRrj8snd7KL31wya4x71jmo5aaSSRmw1dGjWLRmEwWMTccwxgsGFGjM".as_bytes().to_vec());
        let s = OTPSecret::new_totp_secret(&key)?;

        s.next_otp(&key)?;
        Ok(())
    }
}