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

use crate::{
    error::{TiberiusError, TiberiusResult},
    Configuration,
};
use erlang_term::Term;
use std::{
    convert::{TryFrom, TryInto},
    default,
    error::Error,
    net::IpAddr,
    num::NonZeroU32,
    str,
};
use tiberius_dependencies::base64;
use tiberius_dependencies::base64::Engine;

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
        ring::pbkdf2::derive(
            ring::pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(1000).unwrap(),
            SALT.as_bytes(),
            secret,
            &mut key,
        );
        ring::pbkdf2::derive(
            ring::pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(1000).unwrap(),
            SIGN_SALT.as_bytes(),
            secret,
            &mut sign_key,
        );
        Ok(KeyData { key, sign_key })
    }

    type Error = TiberiusError;
}

pub struct CookieData<'a> {
    pub cookie: &'a str,
}

pub struct PhilomenaCookie {
    live_socket_id: Option<String>,
    csrf_token: Option<String>,
    user_token: Option<Vec<u8>>,
}

impl PhilomenaCookie {
    pub fn user_token(&self) -> Option<&[u8]> {
        self.user_token.as_deref()
    }
}

impl TryFrom<Term> for PhilomenaCookie {
    type Error = TiberiusError;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        let value = value.as_map().ok_or(TiberiusError::ErlangTermDecode(
            "Philomena Cookie invalid".to_string(),
        ))?;
        let live_socket_id: Option<String> = value
            .get(&erlang_term::Term::from("live_socket_id"))
            .cloned()
            .and_then(|x| x.as_string());
        let csrf_token: Option<String> = value
            .get(&erlang_term::Term::from("_csrf_token"))
            .cloned()
            .and_then(|x| x.as_string());
        let user_token: Option<Vec<u8>> = value
            .get(&erlang_term::Term::from("user_token"))
            .cloned()
            .and_then(|x| x.as_bytes());
        Ok(PhilomenaCookie {
            live_socket_id,
            csrf_token,
            user_token,
        })
    }
}

impl TryFrom<(&Configuration, &str)> for PhilomenaCookie {
    type Error = TiberiusError;

    fn try_from((config, cookie): (&Configuration, &str)) -> Result<Self, Self::Error> {
        request_cookie_data(config, cookie)
    }
}

pub(crate) const PHOENIX_AAD: [u8; 7] = *b"A128GCM";
const SALT: &str = "authenticated encrypted cookie";
const SIGN_SALT: &str = "signed cookie";

pub fn request_authenticated(c: &Configuration, cookie: &str) -> TiberiusResult<bool> {
    let key: KeyData = c.clone().try_into()?;
    determine(&key, cookie)
}

pub fn ip_authenticated(c: &Configuration, cookie: &str, ip: &IpAddr) -> TiberiusResult<bool> {
    let key: KeyData = c.clone().try_into()?;
    determine_ip(&key, cookie, ip)
}

pub fn request_cookie_data(
    config: &Configuration,
    cookie: &str,
) -> TiberiusResult<PhilomenaCookie> {
    let key_data: KeyData = KeyData::try_from(config.clone())?;
    let term = decode(&key_data, cookie)?;
    PhilomenaCookie::try_from(term)
}

fn decode(key: &KeyData, cookie: &str) -> TiberiusResult<Term> {
    let decoded = decode_cookie(cookie)?;
    let cek = unwrap_cek(key, &decoded)?;
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

#[allow(clippy::diverging_sub_expression)]
fn determine(key: &KeyData, cookie: &str) -> TiberiusResult<bool> {
    let term = decode(key, cookie)?;
    let decrypted: &[u8] = todo!();
    let determined = session_important(decrypted);

    Ok(determined)
}

#[allow(clippy::diverging_sub_expression)]
fn determine_ip(key: &KeyData, cookie: &str, ip: &IpAddr) -> TiberiusResult<bool> {
    let term = decode(key, cookie)?;
    let decrypted: &[u8] = todo!();
    let important = session_important(decrypted);
    let determined = important && contains_ip(decrypted, ip);

    Ok(determined)
}

fn decode_cookie(cookie: &str) -> TiberiusResult<ElixirCookie> {
    let parts: Vec<&str> = cookie.split('.').collect();

    if parts.len() != 5 {
        return Err(TiberiusError::InvalidPhilomenaCookie);
    }

    let aad = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[0])?;
    let cek = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1])?;
    let iv = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[2])?;
    let data = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[3])?;
    let auth_tag = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[4])?;

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

pub(crate) fn unwrap_cek(key: &KeyData, cookie: &ElixirCookie) -> TiberiusResult<Vec<u8>> {
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
    let cek = {
        use ring::aead::*;
        let ukey = UnboundKey::new(&ring::aead::AES_256_GCM, &key.key)?;
        let lskey = LessSafeKey::new(ukey);
        let sign_key = Aad::from(&key.sign_key);
        let cipher_text_and_tag = &cookie.cek[0..32].to_vec();
        let mut cipher_text_and_tag = cipher_text_and_tag.clone();
        let nonce = Nonce::try_assume_unique_for_key(iv)?;
        lskey.open_in_place(nonce, sign_key, &mut cipher_text_and_tag)?;
        cipher_text_and_tag[0..16].to_vec()
    };
    Ok(cek)
}

fn decrypt_session(
    cek: &[u8],
    aad: &[u8],
    iv: &[u8],
    data: &[u8],
    auth_tag: &[u8],
) -> TiberiusResult<Vec<u8>> {
    use ring::aead::*;
    let cek = LessSafeKey::new(UnboundKey::new(&AES_128_GCM, cek)?);
    let nonce = Nonce::try_assume_unique_for_key(iv)?;
    let aad = Aad::from(aad);
    let mut in_out = data.to_vec();
    in_out.extend_from_slice(auth_tag);
    let out = cek.open_in_place(nonce, aad, &mut in_out)?;
    Ok(out.to_vec())
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

    use tiberius_dependencies::hex;

    use super::unwrap_cek;
    use std::convert::TryInto;

    use crate::{
        config::Configuration,
        error::TiberiusResult,
        session::philomena_plug::session::{
            decode_cookie, decrypt_session, request_authenticated, ElixirCookie, KeyData,
        },
    };

    fn config() -> Configuration {
        Configuration {
            philomena_secret: Some(
                "LpXXqV073a8rUzW1k+CkL9/th3qFJL5VhaKYoNYZtXA5+C0M/cZHpgVaEbagYE40".to_string(),
            ),
            ..Default::default()
        }
    }

    /*#[test]
    fn test_cek_key_derive() -> TiberiusResult<()> {
        let config = config();
        let secret = b"LpXXqV073a8rUzW1k+CkL9/th3qFJL5VhaKYoNYZtXA5+C0M/cZHpgVaEbagYE40";
        let salt = b"authenticated encrypted cookie";
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
        let wrapped_cek = base64::decode_config(
            "NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA",
            base64::URL_SAFE_NO_PAD,
        )
        .unwrap();
        let unwrapped_cek_c = crate::session::philomena_plug::session_c::unwrap_cek(
            &key_data_c,
            &wrapped_cek.clone(),
        )
        .unwrap_or_default();
        let unwrapped_cek = unwrap_cek(
            &key_data,
            &ElixirCookie {
                aad: Vec::new(),
                cek: wrapped_cek.clone(),
                iv: Vec::new(),
                data: Vec::new(),
                auth_tag: Vec::new(),
            },
        )
        .unwrap_or_default();
        assert_eq!(unwrapped_cek_c, unwrapped_cek);
        assert!(unwrapped_cek_c.len() > 0);
        assert!(unwrapped_cek.len() > 0);
        Ok(())
    }*/

    #[test]
    fn test_cookie_decode() -> TiberiusResult<()> {
        let config = config();
        let key_data: KeyData = config.try_into().unwrap();
        assert_eq!(key_data, {
            let key =
                hex::decode("845a1b1b9f9c6e1e124bfd9d284c48b18c679491f72f4e9c5359dfb2b816402f")
                    .unwrap();
            let key = {
                let mut keya: [u8; 32] = [0; 32];
                keya[..32].copy_from_slice(&key[..32]);
                keya
            };
            let sign_key =
                hex::decode("6fb3acca739190a8f99e151c98c8dc0a7bc6fa672786542b5be737c75e96aa03")
                    .unwrap();
            let sign_key = {
                let mut sign_keya: [u8; 32] = [0; 32];
                sign_keya[..32].copy_from_slice(&sign_key[..32]);
                sign_keya
            };
            KeyData { key, sign_key }
        });
        let cookie = r#"QTEyOEdDTQ.NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA.93XGZOc41D1VQuLE.rhjFuaKWSBVzLbg-pFUfUGW3TuXi0-_eU_Nypvhy4c1UcuDWMzoR9ojJEWVuwbp9Tj53aNHm3hi8gtatVoxx6v8L9Jgl3Ot9e9LMb5MY27Jk-1vnF6qgNOqo2ScBZ96laWUOro4ZIP8CNH_YMypDQaIJQRXqjNAEjodLjSxGfEYNKiQffE5ma6aa8BAyll77Yi5-u5u8_RsUbVNqADDmboJKjrIskEg45fVR6M4xedmTbuAMD72jbII8.N7CR_qyW5nCaWB6ZdP5org"#;
        let cookie = decode_cookie(cookie)?;
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD as b64c, Engine};
        use tiberius_dependencies::base64;
        let expected = ElixirCookie{
            aad: (super::PHOENIX_AAD.to_vec()),
            cek: b64c.decode("NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA").unwrap(),
            iv: b64c.decode("93XGZOc41D1VQuLE").unwrap(),
            data: b64c.decode("rhjFuaKWSBVzLbg-pFUfUGW3TuXi0-_eU_Nypvhy4c1UcuDWMzoR9ojJEWVuwbp9Tj53aNHm3hi8gtatVoxx6v8L9Jgl3Ot9e9LMb5MY27Jk-1vnF6qgNOqo2ScBZ96laWUOro4ZIP8CNH_YMypDQaIJQRXqjNAEjodLjSxGfEYNKiQffE5ma6aa8BAyll77Yi5-u5u8_RsUbVNqADDmboJKjrIskEg45fVR6M4xedmTbuAMD72jbII8").unwrap(),
            auth_tag: b64c.decode("N7CR_qyW5nCaWB6ZdP5org").unwrap()
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
        let term = erlang_term::Term::from_bytes(&decrypted).unwrap();
        println!("{:?}", term);
        let term = term.as_map().expect("must be a toplevel map");
        assert!(
            term.contains_key(&erlang_term::Term::from("live_socket_id")),
            "{term:?} contains live socket id"
        );
        assert!(
            term.contains_key(&erlang_term::Term::from("_csrf_token")),
            "{term:?} contains csrf token"
        );
        assert!(
            term.contains_key(&erlang_term::Term::from("user_token")),
            "{term:?} contains user token"
        );
        Ok(())
    }

    #[test]
    fn test_full_session_decode() -> TiberiusResult<()> {
        use std::convert::TryFrom;
        let config = config();
        let key_data: KeyData = config.clone().try_into().unwrap();
        let cookie = r#"QTEyOEdDTQ.NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA.93XGZOc41D1VQuLE.rhjFuaKWSBVzLbg-pFUfUGW3TuXi0-_eU_Nypvhy4c1UcuDWMzoR9ojJEWVuwbp9Tj53aNHm3hi8gtatVoxx6v8L9Jgl3Ot9e9LMb5MY27Jk-1vnF6qgNOqo2ScBZ96laWUOro4ZIP8CNH_YMypDQaIJQRXqjNAEjodLjSxGfEYNKiQffE5ma6aa8BAyll77Yi5-u5u8_RsUbVNqADDmboJKjrIskEg45fVR6M4xedmTbuAMD72jbII8.N7CR_qyW5nCaWB6ZdP5org"#;
        let phck = super::PhilomenaCookie::try_from((&config, cookie))?;
        assert_eq!(
            "users_sessions:v3qg6KrwisBK6sM1iYiw_eW6HcbFXgb5qU0-SHgDL48=",
            phck.live_socket_id.unwrap()
        );
        assert_eq!("HDPbiDZTSWZIWgCGruaFXfOz", phck.csrf_token.unwrap());
        assert_eq!(
            "bf7aa0e8aaf08ac04aeac3358988b0fde5ba1dc6c55e06f9a94d3e4878032f8f",
            hex::encode(phck.user_token.unwrap())
        );
        Ok(())
    }
}
