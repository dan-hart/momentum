// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Nextcloud password in the Secret Service (or the portal-backed file keyring when sandboxed).
const ATTRS: [(&str, &str); 2] = [("app", "io.github.dan_hart.Momentum"), ("purpose", "nextcloud")];

pub fn get() -> Option<String> {
    async_io::block_on(async {
        let k = oo7::Keyring::new().await.ok()?;
        let item = k.search_items(&ATTRS).await.ok()?.into_iter().next()?;
        String::from_utf8(item.secret().await.ok()?.as_bytes().to_vec()).ok()
    })
}
pub fn set(password: &str) -> Result<(), oo7::Error> {
    async_io::block_on(async {
        oo7::Keyring::new()
            .await?
            .create_item("Momentum Nextcloud password", &ATTRS, password, true)
            .await
    })
}
