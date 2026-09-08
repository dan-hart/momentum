// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! End-to-end encryption compatible with upstream `packages/sync-core/src/encryption*`:
//! Argon2id (64 MiB, t=3, p=1) → AES-256-GCM, wire = base64(salt16 ‖ iv12 ‖ ciphertext+tag).
//! Legacy files use PBKDF2-HMAC-SHA256 (1000 rounds, salt = password) and base64(iv12 ‖ ct).
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use std::collections::HashMap;
use std::sync::Mutex;

const SALT: usize = 16;
const IV: usize = 12;
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;
static KEYS: Mutex<Option<HashMap<(String, Vec<u8>), [u8; 32]>>> = Mutex::new(None);

fn derive(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let cache_key = (password.to_string(), salt.to_vec());
    if let Some(k) = KEYS.lock().unwrap().get_or_insert_with(HashMap::new).get(&cache_key) {
        return Ok(*k);
    }
    let params = argon2::Params::new(65536, 3, 1, Some(32)).map_err(|e| e.to_string())?;
    let mut out = [0u8; 32];
    argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|e| e.to_string())?;
    KEYS.lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(cache_key, out);
    Ok(out)
}

pub fn encrypt(plain: &str, password: &str) -> Result<String, String> {
    let mut salt = [0u8; SALT];
    let mut iv = [0u8; IV];
    getrandom::fill(&mut salt).map_err(|e| e.to_string())?;
    getrandom::fill(&mut iv).map_err(|e| e.to_string())?;
    let key = derive(password, &salt)?;
    let ct = Aes256Gcm::new_from_slice(&key)
        .unwrap()
        .encrypt(&Nonce::try_from(&iv[..]).unwrap(), plain.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::with_capacity(SALT + IV + ct.len());
    buf.extend_from_slice(&salt);
    buf.extend_from_slice(&iv);
    buf.extend_from_slice(&ct);
    Ok(B64.encode(buf))
}

pub fn decrypt(data: &str, password: &str) -> Result<String, String> {
    let buf = B64.decode(data.trim()).map_err(|e| e.to_string())?;
    if buf.len() >= SALT + IV + 16 {
        if let Ok(key) = derive(password, &buf[..SALT]) {
            if let Ok(pt) = Aes256Gcm::new_from_slice(&key)
                .unwrap()
                .decrypt(&Nonce::try_from(&buf[SALT..SALT + IV]).unwrap(), &buf[SALT + IV..])
            {
                return String::from_utf8(pt).map_err(|e| e.to_string());
            }
        }
    }
    if buf.len() < IV + 16 {
        return Err("encrypted data is too short".into());
    }
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), password.as_bytes(), 1000, &mut key);
    let pt = Aes256Gcm::new_from_slice(&key)
        .unwrap()
        .decrypt(&Nonce::try_from(&buf[..IV]).unwrap(), &buf[IV..])
        .map_err(|_| "wrong encryption password".to_string())?;
    String::from_utf8(pt).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_reject_wrong_password() {
        let c = encrypt("{\"hello\":1}", "pw").unwrap();
        assert_eq!(decrypt(&c, "pw").unwrap(), "{\"hello\":1}");
        assert!(decrypt(&c, "nope").is_err());
    }
}
