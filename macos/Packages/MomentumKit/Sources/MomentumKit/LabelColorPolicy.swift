// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
public enum LabelColorPolicy {
    public static func allowsColor(preference: Bool, increasedContrast: Bool, differentiateWithoutColor: Bool) -> Bool {
        preference && !increasedContrast && !differentiateWithoutColor
    }
}
