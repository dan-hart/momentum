// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit

extension TaskFormModel {
    public var mobileEstimateInvalid: Bool {
        !estimateText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && parseEstimate(text: estimateText) == nil
    }

    public var mobileCanSave: Bool { canSave && !timeInvalid && !mobileEstimateInvalid }
}
