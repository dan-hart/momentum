<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Nextcloud Client Push privacy and lifecycle

Client Push is an optional acceleration hint. Periodic WebDAV reconciliation remains
authoritative because notifications can be missing, duplicated or unrelated to tasks.
No server setup or administration is performed by the client.

The shared Rust listener discovers the configured server's authenticated OCS capability
and accepts only an HTTPS server with a WSS endpoint on the same origin. It verifies
TLS certificates, refuses redirects and URL credentials, sends the account username
and password according to the [Nextcloud protocol](https://github.com/nextcloud/notify_push/blob/main/DEVELOPING.md),
and accepts file hints only after the server confirms authentication.

The listener emits no credential, server-address or message-payload logs. The `log`
crate has a compile-time **Info** ceiling across Rust consumers: Tungstenite's Debug
and Trace messages otherwise format authentication frames and peer message payloads.
Those messages stay unavailable even with a verbose runtime logging filter. Ordinary
Info/Warn/Error diagnostics remain available; the selected WebSocket dependency does
not format frames at those levels. This ceiling also removes unrelated dependency
Debug/Trace output, an intentional privacy tradeoff.

Hints coalesce into one atomic flag. Frames are limited to 16 KiB; connection and
authentication phases have bounded deadlines, retries back off, and unsupported
capabilities are cached briefly. Provider/configuration changes and native lifecycle
invalidation close the socket. An abandoned listener expires after a 180-second
activity lease, including when a peer trickles incomplete frames. The session owner
keeps only a configuration fingerprint; worker credentials expire with the worker.
An in-progress operating-system DNS lookup cannot be interrupted, so cancelled
worker handles are retained until they finish rather than spawning unlimited workers.

Isolated tests exercise trusted HTTPS/WSS discovery and authentication, rejected
certificate roots, redirects, unsupported capabilities, oversized/partial frames,
backoff, cancellation, configuration changes, lease expiry and the logging ceiling.
Actual server support, device background wakes and native scheduling remain separate
acceptance checks.

The optional durable legacy baseline binds only nonsecret transport configuration,
the exact encoded payload digest, server revision and strong ETag. It never stores
a password-derived fast fingerprint. Loading an encrypted payload still uses its
existing authenticated encryption and strong password KDF; conditional validation
still authenticates to the server with the current HTTP credentials. Process-local
cache reuse additionally requires exact current credentials and encryption settings.
