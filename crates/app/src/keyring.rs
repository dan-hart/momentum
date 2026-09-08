// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Secrets in the Secret Service (or the portal-backed file keyring when sandboxed).
fn attrs(purpose: &'static str) -> [(&'static str, &'static str); 2] {
    [("app", "io.github.dan_hart.Momentum"), ("purpose", purpose)]
}

pub fn get(purpose: &'static str) -> Option<String> {
    async_io::block_on(async {
        let k = oo7::Keyring::new().await.ok()?;
        let item = k.search_items(&attrs(purpose)).await.ok()?.into_iter().next()?;
        String::from_utf8(item.secret().await.ok()?.as_bytes().to_vec()).ok()
    })
}
pub fn set(purpose: &'static str, secret: &str) -> Result<(), oo7::Error> {
    async_io::block_on(async {
        oo7::Keyring::new()
            .await?
            .create_item(&format!("Momentum {purpose}"), &attrs(purpose), secret, true)
            .await
    })
}
