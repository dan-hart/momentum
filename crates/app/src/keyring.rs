// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Secrets in the Secret Service (or the portal-backed file keyring when sandboxed).
fn attrs(purpose: &str) -> [(&'static str, &str); 2] {
    [("app", "io.github.dan_hart.Momentum"), ("purpose", purpose)]
}

pub fn get_named(purpose: &str) -> Option<String> {
    async_io::block_on(async {
        let k = oo7::Keyring::new().await.ok()?;
        let item = k.search_items(&attrs(purpose)).await.ok()?.into_iter().next()?;
        String::from_utf8(item.secret().await.ok()?.as_bytes().to_vec()).ok()
    })
}
pub fn set_named(purpose: &str, secret: &str) -> Result<(), oo7::Error> {
    async_io::block_on(async {
        oo7::Keyring::new()
            .await?
            .create_item(&format!("Momentum {purpose}"), &attrs(purpose), secret, true)
            .await
    })
}
pub fn delete_named(purpose: &str) {
    async_io::block_on(async {
        if let Ok(k) = oo7::Keyring::new().await {
            k.delete(&attrs(purpose)).await.ok();
        }
    });
}
pub fn get(purpose: &'static str) -> Option<String> {
    get_named(purpose)
}
pub fn set(purpose: &'static str, secret: &str) -> Result<(), oo7::Error> {
    set_named(purpose, secret)
}
