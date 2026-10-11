// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.app.Application
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.platform.PlatformCoordinator

class MomentumApplication : Application() {
    // Workers, activities and notification actions share this engine owner.
    val repository by lazy { MomentumRepository(this) }
    val platform by lazy { PlatformCoordinator(this, repository) }
}
