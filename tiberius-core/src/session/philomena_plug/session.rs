/*
MIT License

Copyright (c) 2017 Liam P. White

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
 */

use crate::error::{TiberiusError, TiberiusResult};
use crate::Configuration;
use erlang_term::Term;
use openssl::hash::MessageDigest;
use openssl::symm::Cipher;
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::net::IpAddr;
use std::{default, str};

/// This Session Plugin allows using Philomena Session Plugins for login if the appropriate session secret keys are
/// present

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct KeyData {
    pub key: [u8; 32],
    pub sign_key: [u8; 32],
}

#[derive(PartialEq, Eq, Debug)]
pub struct ElixirCookie {
    pub aad: Vec<u8>,
    pub cek: Vec<u8>,
    pub iv: Vec<u8>,
    pub data: Vec<u8>,
    pub auth_tag: Vec<u8>,
}

impl TryFrom<Configuration> for KeyData {
    fn try_from(c: Configuration) -> TiberiusResult<Self> {
        let secret = c.philomena_secret();
        let secret = match secret {
            None => {
                return TiberiusResult::Err(TiberiusError::ConfigurationUnset(
                    "PHILOMENA_SECRET".to_string(),
                ))
            }
            Some(v) => v.as_bytes(),
        };
        let mut sign_key: [u8; 32] = [0; 32];
        let mut key: [u8; 32] = [0; 32];
        openssl::pkcs5::pbkdf2_hmac(
            &secret,
            SALT.as_bytes(),
            1000,
            MessageDigest::sha256(),
            &mut key,
        )?;
        openssl::pkcs5::pbkdf2_hmac(
            &secret,
            SIGN_SALT.as_bytes(),
            1000,
            MessageDigest::sha256(),
            &mut sign_key,
        )?;
        Ok(KeyData { key, sign_key })
    }

    type Error = TiberiusError;
}

#[repr(C)]
pub struct CookieData<'a> {
    pub cookie: &'a str,
}

pub(crate) const PHOENIX_AAD: [u8; 7] = *b"A128GCM";
const SALT: &'static str = "signed encrypted cookie";
const SIGN_SALT: &'static str = "signed cookie";

pub fn request_authenticated<'a>(c: &Configuration, cookie: &str) -> TiberiusResult<bool> {
    let key: KeyData = c.clone().try_into()?;
    determine(&key, cookie)
}

pub fn ip_authenticated<'a>(c: &Configuration, cookie: &str, ip: &IpAddr) -> TiberiusResult<bool> {
    let key: KeyData = c.clone().try_into()?;
    determine_ip(&key, cookie, ip)
}

fn decode<'a>(key: &KeyData, cookie: &str) -> TiberiusResult<Term> {
    let decoded = decode_cookie(cookie)?;
    let cek = unwrap_cek(&key, &decoded)?;
    let decrypted = decrypt_session(
        &cek,
        &decoded.aad,
        &decoded.iv,
        &decoded.data,
        &decoded.auth_tag,
    )?;
    let term = Term::from_bytes(&decrypted);
    let term = match term {
        Err(v) => return Err(TiberiusError::ErlangTermDecode(v.to_string())),
        Ok(v) => v,
    };
    debug!("Philomena Session Data: {:?}", term);
    Ok(term)
}

fn determine<'a>(key: &KeyData, cookie: &str) -> TiberiusResult<bool> {
    let term = decode(key, cookie)?;
    let decrypted: &[u8] = todo!();
    let determined = session_important(&decrypted);

    Ok(determined)
}

fn determine_ip<'a>(key: &KeyData, cookie: &str, ip: &IpAddr) -> TiberiusResult<bool> {
    let term = decode(key, cookie)?;
    let decrypted: &[u8] = todo!();
    let important = session_important(&decrypted);
    let determined = important && contains_ip(&decrypted, ip);

    Ok(determined)
}

fn decode_cookie<'a>(cookie: &str) -> TiberiusResult<ElixirCookie> {
    let parts: Vec<&str> = cookie.split(".").collect();

    if parts.len() != 5 {
        return Err(TiberiusError::InvalidPhilomenaCookie);
    }

    let aad = base64::decode_config(parts[0], base64::URL_SAFE_NO_PAD)?;
    let cek = base64::decode_config(parts[1], base64::URL_SAFE_NO_PAD)?;
    let iv = base64::decode_config(parts[2], base64::URL_SAFE_NO_PAD)?;
    let data = base64::decode_config(parts[3], base64::URL_SAFE_NO_PAD)?;
    let auth_tag = base64::decode_config(parts[4], base64::URL_SAFE_NO_PAD)?;

    if !aad.eq(&PHOENIX_AAD) || cek.len() != 44 || iv.len() != 12 || auth_tag.len() != 16 {
        return Err(TiberiusError::InvalidPhilomenaCookie);
    }

    Ok(ElixirCookie {
        aad,
        cek,
        iv,
        data,
        auth_tag,
    })
}

pub(crate) fn unwrap_cek<'a>(key: &KeyData, cookie: &ElixirCookie) -> TiberiusResult<Vec<u8>> {
    debug_assert!(
        cookie.cek.len() == 44,
        "CEK must be 44 bytes, is {}",
        cookie.cek.len()
    );
    debug_assert!(
        key.sign_key.len() * 8 == 256
            || key.sign_key.len() * 8 == 192
            || key.sign_key.len() * 8 == 128,
        "SIGN KEY must be 256 bit, was {}",
        key.sign_key.len() * 8
    );
    debug_assert!(
        key.key.len() * 8 == 256 || key.key.len() * 8 == 192 || key.key.len() * 8 == 128,
        "SIGN KEY must be 256 bit, was {}",
        key.key.len() * 8
    );
    let cipher_text = &cookie.cek[0..16]; // 128 bit data
    let cipher_tag = &cookie.cek[16..32]; // 128 bit AEAD tag
    let iv = &cookie.cek[32..44]; // 96 bit IV
    let cek = openssl::symm::decrypt_aead(
        Cipher::aes_256_gcm(),
        &key.key,
        Some(iv),
        &key.sign_key,
        cipher_text,
        cipher_tag,
    );
    let cek = match cek {
        Ok(cek) => cek,
        Err(e) => {
            warn!("errors: {:?}", e);
            return Err(TiberiusError::OpenSSLComplex(e));
        }
    };
    Ok(cek)
}

fn decrypt_session(
    cek: &Vec<u8>,
    aad: &Vec<u8>,
    iv: &Vec<u8>,
    data: &Vec<u8>,
    auth_tag: &Vec<u8>,
) -> TiberiusResult<Vec<u8>> {
    Ok(openssl::symm::decrypt_aead(
        Cipher::aes_128_gcm(),
        &cek,
        Some(&iv),
        &aad,
        &data,
        &auth_tag,
    )?)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn session_important(session_data: &[u8]) -> bool {
    find_subsequence(session_data, b"user_token").is_some()
}

fn contains_ip(session_data: &[u8], ip: &IpAddr) -> bool {
    let ip = ip.to_string();
    let ip = ip.as_bytes();
    find_subsequence(session_data, ip).is_some()
}

#[cfg(test)]
mod test {

    use super::unwrap_cek;
    use std::convert::TryInto;

    use crate::config::Configuration;
    use crate::error::TiberiusResult;
    use crate::session::philomena_plug::session::{
        decode_cookie, decrypt_session, request_authenticated, ElixirCookie, KeyData,
    };

    fn config() -> Configuration {
        Configuration {
            philomena_secret: Some(
                "LpXXqV073a8rUzW1k+CkL9/th3qFJL5VhaKYoNYZtXA5+C0M/cZHpgVaEbagYE40".to_string(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn test_cek_key_derive() -> TiberiusResult<()> {
        let config = config();
        let secret = b"LpXXqV073a8rUzW1k+CkL9/th3qFJL5VhaKYoNYZtXA5+C0M/cZHpgVaEbagYE40";
        let salt = b"signed encrypted cookie";
        let sign_salt = b"signed cookie";
        let mut key_data_c = super::super::session_c::types::KeyData {
            secret: secret,
            salt: salt,
            sign_salt: sign_salt,
            key: [0; 32],
            sign_key: [0; 32],
        };
        unsafe { crate::session::philomena_plug::session_c::c_derive_key(&mut key_data_c) };
        let key_data: KeyData = config.clone().try_into().unwrap();
        assert_eq!(key_data.key, key_data_c.key);
        assert_eq!(key_data.sign_key, key_data_c.sign_key);
        let wrapped_cek = base64::decode_config("NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA", base64::URL_SAFE_NO_PAD).unwrap();
        let unwrapped_cek_c = crate::session::philomena_plug::session_c::unwrap_cek(&key_data_c, &wrapped_cek.clone()).unwrap_or_default();
        let unwrapped_cek = unwrap_cek(&key_data, &ElixirCookie{
            aad: Vec::new(),
            cek: wrapped_cek.clone(),
            iv: Vec::new(),
            data: Vec::new(),
            auth_tag: Vec::new(),
        }).unwrap_or_default();
        assert_eq!(unwrapped_cek_c, unwrapped_cek);
        assert!(unwrapped_cek_c.len() > 0);
        assert!(unwrapped_cek.len() > 0);
        Ok(())
    }

    #[test]
    fn test_cookie_decode() -> TiberiusResult<()> {
        let config = config();
        let key_data: KeyData = config.clone().try_into().unwrap();
        assert_eq!(key_data, {
            let key =
                hex::decode("92420c1473fd05dcf0bf49b14ac931a0cef24879b81ff8104d0b3eac2165e79c")
                    .unwrap();
            let key = {
                let mut keya: [u8; 32] = [0; 32];
                for i in 0..32 {
                    keya[i] = key[i];
                }
                keya
            };
            let sign_key =
                hex::decode("6fb3acca739190a8f99e151c98c8dc0a7bc6fa672786542b5be737c75e96aa03")
                    .unwrap();
            let sign_key = {
                let mut sign_keya: [u8; 32] = [0; 32];
                for i in 0..32 {
                    sign_keya[i] = sign_key[i];
                }
                sign_keya
            };
            KeyData { key, sign_key }
        });
        let cookie = r#"QTEyOEdDTQ.NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA.93XGZOc41D1VQuLE.rhjFuaKWSBVzLbg-pFUfUGW3TuXi0-_eU_Nypvhy4c1UcuDWMzoR9ojJEWVuwbp9Tj53aNHm3hi8gtatVoxx6v8L9Jgl3Ot9e9LMb5MY27Jk-1vnF6qgNOqo2ScBZ96laWUOro4ZIP8CNH_YMypDQaIJQRXqjNAEjodLjSxGfEYNKiQffE5ma6aa8BAyll77Yi5-u5u8_RsUbVNqADDmboJKjrIskEg45fVR6M4xedmTbuAMD72jbII8.N7CR_qyW5nCaWB6ZdP5org"#;
        let cookie = decode_cookie(cookie)?;
        let expected = ElixirCookie{
            aad: (super::PHOENIX_AAD.to_vec()),
        cek: base64::decode_config("NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA", base64::URL_SAFE_NO_PAD).unwrap(),
        iv: base64::decode_config("93XGZOc41D1VQuLE", base64::URL_SAFE_NO_PAD).unwrap(),
        data: base64::decode_config("rhjFuaKWSBVzLbg-pFUfUGW3TuXi0-_eU_Nypvhy4c1UcuDWMzoR9ojJEWVuwbp9Tj53aNHm3hi8gtatVoxx6v8L9Jgl3Ot9e9LMb5MY27Jk-1vnF6qgNOqo2ScBZ96laWUOro4ZIP8CNH_YMypDQaIJQRXqjNAEjodLjSxGfEYNKiQffE5ma6aa8BAyll77Yi5-u5u8_RsUbVNqADDmboJKjrIskEg45fVR6M4xedmTbuAMD72jbII8", base64::URL_SAFE_NO_PAD).unwrap(),
        auth_tag: base64::decode_config("N7CR_qyW5nCaWB6ZdP5org", base64::URL_SAFE_NO_PAD).unwrap()
        };
        assert_eq!(cookie, expected,);
        let cek = unwrap_cek(&key_data, &cookie).expect("need CEK unwrap");
        let decrypted = decrypt_session(
            &cek,
            &cookie.aad,
            &cookie.iv,
            &cookie.data,
            &cookie.auth_tag,
        )
        .expect("could not decrypt session");
        //let res = request_authenticated(&config, cookie).unwrap();
        todo!()
    }
}
