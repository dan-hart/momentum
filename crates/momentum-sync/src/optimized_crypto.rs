// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! A random vault key wrapped by Argon2id; session owns bounded caching.
use super::*;
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    collection: String,
    vault: String,
    encrypted: bool,
    salt: String,
    wrapped: String,
}
#[derive(Serialize, Deserialize)]
struct Ciphertext {
    version: u32,
    nonce: String,
    body: String,
}
#[derive(Clone)]
pub(super) struct Key {
    collection: String,
    vault: String,
    key: Option<[u8; 32]>,
    cache: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<(String, String, String)>>>,
}
impl Drop for Key {
    fn drop(&mut self) {
        if let Some(key) = self.key.as_mut() {
            for byte in key {
                unsafe { std::ptr::write_volatile(byte, 0) };
            }
            std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
        }
    }
}
fn random<const N: usize>() -> Result<[u8; N], SyncError> {
    let mut out = [0; N];
    getrandom::fill(&mut out).map_err(|_| bad("random source unavailable"))?;
    Ok(out)
}
#[cfg(test)]
thread_local! { static DERIVATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
fn derive(password: &str, salt: &[u8]) -> Result<[u8; 32], SyncError> {
    #[cfg(test)]
    DERIVATIONS.with(|n| n.set(n.get() + 1));
    let params = argon2::Params::new(65536, 3, 1, Some(32)).map_err(|_| bad("invalid KDF parameters"))?;
    let mut key = [0; 32];
    argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| bad("key derivation failed"))?;
    Ok(key)
}
fn encrypt(key: &[u8; 32], aad: &[u8], plain: &[u8]) -> Result<String, SyncError> {
    let nonce = random::<12>()?;
    let ct = Aes256Gcm::new_from_slice(key)
        .unwrap()
        .encrypt(&Nonce::try_from(nonce.as_slice()).unwrap(), Payload { msg: plain, aad })
        .map_err(|_| bad("encryption failed"))?;
    serde_json::to_string(&Ciphertext {
        version: 3,
        nonce: B64.encode(nonce),
        body: B64.encode(ct),
    })
    .map_err(|_| bad("invalid encrypted object"))
}
fn decrypt(key: &[u8; 32], aad: &[u8], body: &str) -> Result<Vec<u8>, SyncError> {
    let c: Ciphertext = serde_json::from_str(body).map_err(|_| bad("invalid encrypted object"))?;
    if c.version != 3 {
        return Err(SyncError::Version(c.version as u64));
    }
    let nonce = B64.decode(c.nonce).map_err(|_| bad("invalid nonce"))?;
    if nonce.len() != 12 {
        return Err(bad("invalid nonce"));
    }
    let ct = B64.decode(c.body).map_err(|_| bad("invalid ciphertext"))?;
    Aes256Gcm::new_from_slice(key)
        .unwrap()
        .decrypt(&Nonce::try_from(nonce.as_slice()).unwrap(), Payload { msg: &ct, aad })
        .map_err(|_| SyncError::WrongEncryptionPassword)
}
fn envelope_aad(e: &Envelope) -> Vec<u8> {
    serde_json::to_vec(&(3, "key-envelope", &e.collection, &e.vault, e.encrypted, &e.salt)).unwrap()
}
pub(super) fn create(collection: &str, password: Option<&str>) -> Result<(String, Key), SyncError> {
    let password = password.filter(|p| !p.is_empty());
    let mut e = Envelope {
        version: 3,
        collection: collection.into(),
        vault: digest(&random::<32>()?),
        encrypted: password.is_some(),
        salt: String::new(),
        wrapped: String::new(),
    };
    let key = if let Some(password) = password {
        e.salt = B64.encode(random::<16>()?);
        let mut wrapping = derive(password, &B64.decode(&e.salt).unwrap())?;
        let key = random::<32>()?;
        let result = encrypt(&wrapping, &envelope_aad(&e), &key);
        for byte in &mut wrapping {
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        e.wrapped = result?;
        Some(key)
    } else {
        None
    };
    let body = serde_json::to_string(&e).map_err(|_| bad("invalid key envelope"))?;
    Ok((
        body,
        Key {
            collection: collection.into(),
            vault: e.vault,
            key,
            cache: Default::default(),
        },
    ))
}
pub(super) fn open(body: &str, collection: &str, password: Option<&str>) -> Result<Key, SyncError> {
    let e: Envelope = serde_json::from_str(body).map_err(|_| bad("invalid key envelope"))?;
    if e.version != 3 {
        return Err(SyncError::Version(e.version as u64));
    }
    if e.collection != collection || e.vault.len() != 64 {
        return Err(bad("key envelope identity mismatch"));
    }
    let key = if e.encrypted {
        let password = password.filter(|p| !p.is_empty()).ok_or(SyncError::Encrypted)?;
        let salt = B64.decode(&e.salt).map_err(|_| bad("invalid key envelope salt"))?;
        if salt.len() != 16 {
            return Err(bad("invalid key envelope salt"));
        }
        let mut wrapping = derive(password, &salt)?;
        let result = decrypt(&wrapping, &envelope_aad(&e), &e.wrapped);
        for byte in &mut wrapping {
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        let mut plain = result?;
        if plain.len() != 32 {
            return Err(bad("invalid vault key"));
        }
        let mut key = [0; 32];
        key.copy_from_slice(&plain);
        for byte in &mut plain {
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        Some(key)
    } else {
        if password.filter(|p| !p.is_empty()).is_some() {
            return Err(bad(
                "this optimized collection was created without encryption; refusing an implicit encryption change",
            ));
        }
        if !e.salt.is_empty() || !e.wrapped.is_empty() {
            return Err(bad("invalid plaintext key envelope"));
        }
        None
    };
    Ok(Key {
        collection: collection.into(),
        vault: e.vault,
        key,
        cache: Default::default(),
    })
}
impl Key {
    pub(super) fn cached(&self, reference: &Object) -> Option<String> {
        self.cache
            .lock()
            .ok()?
            .iter()
            .find(|(id, hash, _)| id == &reference.id && hash == &reference.hash)
            .map(|(_, _, body)| body.clone())
    }
    pub(super) fn remember(&self, reference: &Object, body: &str) {
        const MAX: usize = 16 << 20;
        if body.len() > MAX {
            return;
        }
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|(id, _, _)| id != &reference.id);
            while cache.len() >= 128 || cache.iter().map(|(_, _, b)| b.len()).sum::<usize>() + body.len() > MAX {
                cache.pop_front();
            }
            cache.push_back((reference.id.clone(), reference.hash.clone(), body.into()));
        }
    }
    fn aad(&self, kind: &str, name: &str) -> Vec<u8> {
        serde_json::to_vec(&(3, &self.collection, &self.vault, kind, name)).unwrap()
    }
    pub(super) fn seal(&self, kind: &str, name: &str, plain: &[u8]) -> Result<String, SyncError> {
        match &self.key {
            Some(key) => encrypt(key, &self.aad(kind, name), plain),
            None => String::from_utf8(plain.to_vec()).map_err(|_| bad("invalid object")),
        }
    }
    pub(super) fn unseal(&self, kind: &str, name: &str, body: &str) -> Result<Vec<u8>, SyncError> {
        match &self.key {
            Some(key) => decrypt(key, &self.aad(kind, name), body),
            None => Ok(body.as_bytes().to_vec()),
        }
    }
}

#[cfg(test)]
pub(super) fn kdf_count() -> usize {
    DERIVATIONS.with(|n| n.get())
}
