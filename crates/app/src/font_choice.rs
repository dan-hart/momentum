// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

pub const MONO_FAMILY: &str = "Atkinson Hyperlegible Mono";
pub const DYSLEXIC_FAMILY: &str = "OpenDyslexic";

/// The fourth entry represents an existing font chosen with GTK's installed-font dialog.
pub fn selected_index(saved: &str) -> u32 {
    match saved.trim() {
        "" => 0,
        MONO_FAMILY => 1,
        DYSLEXIC_FAMILY => 2,
        _ => 3,
    }
}

pub fn font_for_index(index: u32) -> Option<&'static str> {
    match index {
        0 => Some(""),
        1 => Some(MONO_FAMILY),
        2 => Some(DYSLEXIC_FAMILY),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_selection_preserves_custom_installed_font_state() {
        assert_eq!(selected_index(""), 0);
        assert_eq!(selected_index(MONO_FAMILY), 1);
        assert_eq!(selected_index(DYSLEXIC_FAMILY), 2);
        assert_eq!(selected_index("Cantarell Bold"), 3);
        assert_eq!(selected_index("Unavailable Custom"), 3);
        assert_eq!(selected_index("Atkinson Hyperlegible Mono Italic"), 3);
    }

    #[test]
    fn curated_selection_changes_only_family_and_custom_is_not_overwritten() {
        assert_eq!(font_for_index(0), Some(""));
        assert_eq!(font_for_index(1), Some(MONO_FAMILY));
        assert_eq!(font_for_index(2), Some(DYSLEXIC_FAMILY));
        assert_eq!(font_for_index(3), None);
        assert_eq!(font_for_index(u32::MAX), None);
    }
}
