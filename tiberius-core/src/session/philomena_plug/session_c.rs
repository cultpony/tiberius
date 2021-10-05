/// This module exists as verification of the non-C module
/// For testing, openssl is linked and the modules does a self-test
/// This code is based on https://github.com/liamwhite/cookie_check and licensed under MIT

/*
 * MIT License
 * 
 * Copyright (c) 2017 Liam P. White
 * 
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 * 
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */


extern crate base64;
extern crate openssl;

use openssl::hash::MessageDigest;
use openssl::symm::Cipher;
use std::error::Error;
use std::str;

pub mod types {
    #[repr(C)]
    pub struct KeyData<'a> {
        pub secret: &'a [u8],
        pub salt: &'a [u8],
        pub sign_salt: &'a [u8],
        pub key: [u8; 32],
        pub sign_key: [u8; 32],
    }

    #[repr(C)]
    pub struct CookieData<'a> {
        pub cookie: &'a [u8],
    }

    #[repr(C)]
    pub struct IpData<'a> {
        pub ip: &'a [u8],
    }
}

use types::*;

const PHOENIX_AAD: [u8; 7] = *b"A128GCM";

#[no_mangle]
pub unsafe extern "C" fn c_request_authenticated(
    key: *const KeyData<'static>,
    cookie: *const CookieData<'static>,
) -> bool {
    determine(&*key, (*cookie).cookie).unwrap_or(false)
}

#[no_mangle]
pub unsafe extern "C" fn c_ip_authenticated(
    key: *const KeyData<'static>,
    cookie: *const CookieData<'static>,
    ip: *const IpData<'static>,
) -> bool {
    determine_ip(&*key, (*cookie).cookie, (*ip).ip).unwrap_or(false)
}

#[no_mangle]
pub unsafe extern "C" fn c_derive_key(key: *mut KeyData<'static>) {
    derive_key(&mut *key).unwrap_or(())
}

// ---

pub fn determine<'a>(key: &KeyData<'a>, cookie: &[u8]) -> Result<bool, Box<dyn Error>> {
    let decoded = decode_cookie(&cookie)?;
    let cek = unwrap_cek(&key, &decoded.1)?;
    let decrypted = decrypt_session(&cek, &decoded.0, &decoded.2, &decoded.3, &decoded.4)?;
    let determined = session_important(&decrypted);

    Ok(determined)
}

pub fn determine_ip<'a>(key: &KeyData<'a>, cookie: &[u8], ip: &[u8]) -> Result<bool, Box<dyn Error>> {
    let decoded = decode_cookie(&cookie)?;
    let cek = unwrap_cek(&key, &decoded.1)?;
    let decrypted = decrypt_session(&cek, &decoded.0, &decoded.2, &decoded.3, &decoded.4)?;
    let important = session_important(&decrypted);
    let determined = important && contains_ip(&decrypted, ip);

    Ok(determined)
}

pub fn decode_cookie<'a>(
    cookie: &[u8],
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>), Box<dyn Error>> {
    let decoded = str::from_utf8(cookie)?;
    let parts: Vec<&str> = decoded.split(".").collect();

    if parts.len() != 5 {
        return Err("invalid cookie".into());
    }

    let aad = base64::decode_config(parts[0], base64::URL_SAFE_NO_PAD)?;
    let cek = base64::decode_config(parts[1], base64::URL_SAFE_NO_PAD)?;
    let iv = base64::decode_config(parts[2], base64::URL_SAFE_NO_PAD)?;
    let data = base64::decode_config(parts[3], base64::URL_SAFE_NO_PAD)?;
    let auth_tag = base64::decode_config(parts[4], base64::URL_SAFE_NO_PAD)?;

    if !aad.eq(&PHOENIX_AAD) || cek.len() != 44 || iv.len() != 12 || auth_tag.len() != 16 {
        return Err("invalid cookie".into());
    }

    Ok((aad, cek, iv, data, auth_tag))
}

pub fn derive_key<'a>(key: &mut KeyData<'a>) -> Result<(), Box<dyn Error>> {
    openssl::pkcs5::pbkdf2_hmac(
        key.secret,
        key.salt,
        1000,
        MessageDigest::sha256(),
        &mut key.key,
    )?;
    openssl::pkcs5::pbkdf2_hmac(
        key.secret,
        key.sign_salt,
        1000,
        MessageDigest::sha256(),
        &mut key.sign_key,
    )?;

    Ok(())
}

pub fn unwrap_cek<'a>(key: &KeyData<'a>, wrapped_cek: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let cipher_text = &wrapped_cek[0..16]; // 128 bit data
    let cipher_tag = &wrapped_cek[16..32]; // 128 bit AEAD tag
    let iv = &wrapped_cek[32..44]; // 96 bit IV

    Ok(openssl::symm::decrypt_aead(
        Cipher::aes_256_gcm(),
        &key.key,
        Some(iv),
        &key.sign_key,
        cipher_text,
        cipher_tag,
    )?)
}

pub fn decrypt_session(
    cek: &Vec<u8>,
    aad: &Vec<u8>,
    iv: &Vec<u8>,
    data: &Vec<u8>,
    auth_tag: &Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(openssl::symm::decrypt_aead(
        Cipher::aes_128_gcm(),
        &cek,
        Some(&iv),
        &aad,
        &data,
        &auth_tag,
    )?)
}

pub fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub fn session_important(session_data: &Vec<u8>) -> bool {
    find_subsequence(session_data, b"user_token").is_some()
}

pub fn contains_ip(session_data: &Vec<u8>, ip: &[u8]) -> bool {
    find_subsequence(session_data, ip).is_some()
}

#[cfg(test)]
mod test {
    use crate::config::Configuration;
    use crate::error::TiberiusResult;
    use crate::session::philomena_plug::session_c::{
        c_derive_key, c_request_authenticated,
        types::{CookieData, KeyData},
    };

    #[test]
    fn test_cookie_decode_c() -> TiberiusResult<()> {
        let cookie = br#"QTEyOEdDTQ.NItJo3ZSO034Y8MKkwONkbq6yAHrDJ5X-4RvNL2g24XS-ycGUaipaViOCHA.93XGZOc41D1VQuLE.rhjFuaKWSBVzLbg-pFUfUGW3TuXi0-_eU_Nypvhy4c1UcuDWMzoR9ojJEWVuwbp9Tj53aNHm3hi8gtatVoxx6v8L9Jgl3Ot9e9LMb5MY27Jk-1vnF6qgNOqo2ScBZ96laWUOro4ZIP8CNH_YMypDQaIJQRXqjNAEjodLjSxGfEYNKiQffE5ma6aa8BAyll77Yi5-u5u8_RsUbVNqADDmboJKjrIskEg45fVR6M4xedmTbuAMD72jbII8.N7CR_qyW5nCaWB6ZdP5org"#;

        let mut key = KeyData {
            secret: b"LpXXqV073a8rUzW1k+CkL9/th3qFJL5VhaKYoNYZtXA5+C0M/cZHpgVaEbagYE40",
            salt: b"authenticated encrypted cookie",
            sign_salt: b"signed cookie",
            key: [0; 32],
            sign_key: [0; 32],
        };

        unsafe { c_derive_key(&mut key) };

        let cookie = CookieData { cookie: cookie };

        assert!(
            unsafe { c_request_authenticated(&key, &cookie) },
            "cookie authenticated"
        );

        Ok(())
    }
}
