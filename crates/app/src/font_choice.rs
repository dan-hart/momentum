// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

pub const MONO_FAMILY: &str = "Atkinson Hyperlegible Mono";
pub const NEXT_FAMILY: &str = "Atkinson Hyperlegible Next";
pub const DYSLEXIC_FAMILY: &str = "OpenDyslexic";

// Keep presentation labels separate from the family names stored in settings.
pub const CURATED_FONTS: [(&str, &str); 3] = [
    ("AH Mono", MONO_FAMILY),
    ("OpenDyslexic", DYSLEXIC_FAMILY),
    ("AH Next", NEXT_FAMILY),
];
/// The final row represents a face chosen with GTK's installed-font dialog.
pub const CUSTOM_INDEX: u32 = CURATED_FONTS.len() as u32 + 1;
pub fn selected_index(saved: &str) -> u32 {
    let saved = saved.trim();
    if saved.is_empty() {
        return 0;
    }
    CURATED_FONTS
        .iter()
        .position(|&(_, family)| family == saved)
        .map_or(CUSTOM_INDEX, |index| index as u32 + 1)
}

pub fn font_for_index(index: u32) -> Option<&'static str> {
    if index == 0 {
        return Some("");
    }
    CURATED_FONTS.get((index - 1) as usize).map(|&(_, family)| family)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_selection_preserves_custom_installed_font_state() {
        assert_eq!(selected_index(""), 0);
        assert_eq!(selected_index(MONO_FAMILY), 1);
        assert_eq!(selected_index(DYSLEXIC_FAMILY), 2);
        assert_eq!(selected_index(NEXT_FAMILY), 3);
        assert_eq!(selected_index("Cantarell Bold"), CUSTOM_INDEX);
        assert_eq!(selected_index("Unavailable Custom"), CUSTOM_INDEX);
        assert_eq!(selected_index("Atkinson Hyperlegible Mono Italic"), CUSTOM_INDEX);
    }

    #[test]
    fn curated_selection_changes_only_family_and_custom_is_not_overwritten() {
        assert_eq!(font_for_index(0), Some(""));
        assert_eq!(font_for_index(1), Some(MONO_FAMILY));
        assert_eq!(font_for_index(2), Some(DYSLEXIC_FAMILY));
        assert_eq!(font_for_index(3), Some(NEXT_FAMILY));
        assert_eq!(font_for_index(CUSTOM_INDEX), None);
        assert_eq!(font_for_index(u32::MAX), None);
    }
}
