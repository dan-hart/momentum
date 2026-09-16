// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import MomentumKit
import SwiftUI

/// Data-defined colors yield to accessibility settings, and update live with the system.
@propertyWrapper
@MainActor
struct LabelColors: DynamicProperty {
    @Environment(AppState.self) private var state
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiate
    var wrappedValue: Bool {
        LabelColorPolicy.allowsColor(preference: state.colorful,
            increasedContrast: contrast == .increased, differentiateWithoutColor: differentiate)
    }
}
