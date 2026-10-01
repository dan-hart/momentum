// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Identity for the sections a list shows. A list diff needs to know that "Completed"
// is a different section from "Today" even when one replaces the other at the same
// position; an offset-based identity told AppKit's outline view the opposite.
import MomentumCore

/// What makes a section itself across two consecutive listings: its kind and group.
/// Every listing the core produces gives each section a distinct identity.
public struct SectionIdentity: Hashable, Sendable {
    public let kind: SectionKind
    public let group: TaskGroup?
}

public extension MomentumCore.Section {
    var identity: SectionIdentity { SectionIdentity(kind: kind, group: group) }
}

public extension Listing {
    /// Every section identity, in order; a test asserts they never repeat.
    var sectionIdentities: [SectionIdentity] { sections.map(\.identity) }
}
