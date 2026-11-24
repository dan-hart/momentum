// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TaskMutationFocusTest {
    @Test fun neighborsFollowOriginalOrderAfterBulkChanges() {
        assertEquals("d", replacementTaskFocus(listOf("a", "b", "c", "d"), listOf("d", "a", "b"), setOf("b", "c")))
        assertEquals("a", replacementTaskFocus(listOf("a", "b"), listOf("a"), setOf("b")))
        assertEquals("b", replacementTaskFocus(listOf("b"), listOf("b"), setOf("b")))
        assertNull(replacementTaskFocus(listOf("b"), emptyList(), setOf("b")))
    }
}
