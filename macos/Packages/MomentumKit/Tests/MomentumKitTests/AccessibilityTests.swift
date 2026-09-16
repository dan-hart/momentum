// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Testing
@testable import MomentumKit

@Suite struct AccessibilityPreferences {
    @Test func systemAccessibilityOverridesCustomLabelColors() {
        #expect(LabelColorPolicy.allowsColor(preference: true, increasedContrast: false, differentiateWithoutColor: false))
        #expect(!LabelColorPolicy.allowsColor(preference: false, increasedContrast: false, differentiateWithoutColor: false))
        #expect(!LabelColorPolicy.allowsColor(preference: true, increasedContrast: true, differentiateWithoutColor: false))
        #expect(!LabelColorPolicy.allowsColor(preference: true, increasedContrast: false, differentiateWithoutColor: true))
    }
}
