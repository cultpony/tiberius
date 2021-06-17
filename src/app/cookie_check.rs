// Original from https://github.com/liamwhite/cookie_check
// Credit to LiamWhite, MIT License applies

use openssl::hash::MessageDigest;
use openssl::symm::Cipher;
use std::str;

use crate::error::TiberiusResult;

const PHOENIX_AAD: [u8; 7] = *b"A128GCM";

#[derive(Clone, securefmt::Debug)]
pub struct KeyData {
    #[sensitive]
    secret: Vec<u8>,
    #[sensitive]
    salt: Vec<u8>,
    #[sensitive]
    sign_salt: Vec<u8>,
    #[sensitive]
    key: [u8; 32],
    #[sensitive]
    sign_key: [u8; 32],
}

pub struct AAD(Vec<u8>);
pub struct WCEK(Vec<u8>);
pub struct CEK(Vec<u8>);
pub struct IV(Vec<u8>);
pub struct AuthTag(Vec<u8>);

impl KeyData {
    pub fn new(secret: Vec<u8>, salt: Vec<u8>, sign_salt: Vec<u8>) -> TiberiusResult<Self> {
        let key = {
            let mut key: [u8; 32] = [0; 32];
            openssl::pkcs5::pbkdf2_hmac(&secret, &salt, 1000, MessageDigest::sha256(), &mut key)?;
            key
        };
        let sign_key = {
            let mut sign_key: [u8; 32] = [0; 32];
            openssl::pkcs5::pbkdf2_hmac(
                &secret,
                &sign_salt,
                1000,
                MessageDigest::sha256(),
                &mut sign_key,
            )?;
            sign_key
        };
        Ok(Self {
            secret,
            salt,
            sign_salt,
            key,
            sign_key,
        })
    }

    pub fn new_str(secret: &str, salt: &str, sign_salt: &str) -> TiberiusResult<Self> {
        Self::new(
            secret.as_bytes().to_vec(),
            salt.as_bytes().to_vec(),
            sign_salt.as_bytes().to_vec(),
        )
    }

    pub fn decrypt_and_verify_cookie(&self, cookie: &[u8]) -> TiberiusResult<Vec<u8>> {
        let (aad, wcek, iv, data, auth_tag) = Self::decode_cookie(&cookie)?;
        let cek = self.unwrap_cek(&wcek)?;
        let decrypted = Self::decrypt_session(&cek, &aad, &iv, &data, &auth_tag)?;
        Ok(decrypted)
    }

    fn decrypt_session(
        cek: &CEK,
        aad: &AAD,
        iv: &IV,
        data: &Vec<u8>,
        auth_tag: &AuthTag,
    ) -> TiberiusResult<Vec<u8>> {
        Ok(openssl::symm::decrypt_aead(
            Cipher::aes_128_gcm(),
            &cek.0,
            Some(&iv.0),
            &aad.0,
            &data,
            &auth_tag.0,
        )?)
    }

    fn decode_cookie(cookie: &[u8]) -> TiberiusResult<(AAD, WCEK, IV, Vec<u8>, AuthTag)> {
        let decoded = str::from_utf8(cookie)?;
        let parts: Vec<&str> = decoded.split(".").collect();

        if parts.len() != 5 {
            anyhow::bail!("invalid cookie");
        }

        let aad = AAD(base64::decode_config(parts[0], base64::URL_SAFE_NO_PAD)?);
        let cek = WCEK(base64::decode_config(parts[1], base64::URL_SAFE_NO_PAD)?);
        let iv = IV(base64::decode_config(parts[2], base64::URL_SAFE_NO_PAD)?);
        let data = base64::decode_config(parts[3], base64::URL_SAFE_NO_PAD)?;
        let auth_tag = AuthTag(base64::decode_config(parts[4], base64::URL_SAFE_NO_PAD)?);

        if !aad.0.eq(&PHOENIX_AAD)
            || cek.0.len() != 44
            || iv.0.len() != 12
            || auth_tag.0.len() != 16
        {
            anyhow::bail!("invalid cookie");
        }

        Ok((aad, cek, iv, data, auth_tag))
    }

    fn unwrap_cek(&self, wrapped_cek: &WCEK) -> TiberiusResult<CEK> {
        let cipher_text = &wrapped_cek.0[0..16]; // 128 bit data
        let cipher_tag = &wrapped_cek.0[16..32]; // 128 bit AEAD tag
        let iv = &wrapped_cek.0[32..44]; // 96 bit IV

        Ok(CEK(openssl::symm::decrypt_aead(
            Cipher::aes_256_gcm(),
            &self.key,
            Some(iv),
            &self.sign_key,
            cipher_text,
            cipher_tag,
        )?))
    }
}
