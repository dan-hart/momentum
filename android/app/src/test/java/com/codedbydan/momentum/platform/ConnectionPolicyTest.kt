// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import org.junit.Assert.*
import org.junit.Test

class ConnectionPolicyTest {
    @Test fun remoteServersRequireHttps() {
        assertTrue(ConnectionPolicy.accepts("https://cloud.example.org/dav", isolated = false))
        assertFalse(ConnectionPolicy.accepts("http://cloud.example.org", isolated = false))
        assertFalse(ConnectionPolicy.accepts("https://user:secret@cloud.example.org", isolated = false))
        assertFalse(ConnectionPolicy.accepts("https://cloud.example.org/#token", isolated = false))
        assertFalse(ConnectionPolicy.accepts("https://cloud.example.org:0", isolated = false))
    }

    @Test fun knownLanHostsNeedLocalNetworkPermissionWithoutResolvingDns() {
        listOf("https://192.168.1.1", "https://10.0.0.2", "https://172.16.0.1", "https://[fd00::1]", "https://cloud.local")
            .forEach { assertTrue(ConnectionPolicy.isLocalServer(it)) }
        assertFalse(ConnectionPolicy.isLocalServer("https://172.32.0.1"))
        assertFalse(ConnectionPolicy.isLocalServer("https://cloud.example.org"))
    }

    @Test fun cleartextLoopbackIsRestrictedToDisposableStores() {
        assertTrue(ConnectionPolicy.accepts("http://127.0.0.1:1234", isolated = true))
        assertTrue(ConnectionPolicy.accepts("http://[::1]:1234", isolated = true))
        assertFalse(ConnectionPolicy.accepts("http://127.0.0.1:1234", isolated = false))
        assertFalse(ConnectionPolicy.accepts("http://localhost.example.org", isolated = true))
        assertFalse(ConnectionPolicy.accepts("http://192.168.1.1", isolated = true))
        assertFalse(ConnectionPolicy.accepts("file:///tmp/cloud", isolated = true))
    }
}
