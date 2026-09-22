// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

use adw::prelude::AdwDialogExt;
use gtk::glib::translate::IntoGlib;
use gtk::pango::{FontDescription, FontMask, Stretch, Style, Variant};
use gtk::prelude::*;
use std::cell::RefCell;

pub const MIN_SCALE: i32 = 75;
pub const MAX_SCALE: i32 = 250;
pub const DEFAULT_SCALE: i32 = 100;
pub const SCALE_STEP: i32 = 5;
pub const INTERFACE_CLASS: &str = "momentum-interface";
pub const CONTENT_CLASS: &str = "momentum-content";

struct ProviderState {
    provider: gtk::CssProvider,
    css: String,
}

thread_local! {
    static PROVIDER: RefCell<Option<ProviderState>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypographyPreferences {
    font: Option<FontDescription>,
    content_scale: i32,
    interface_scale: i32,
}

impl TypographyPreferences {
    pub fn from_persisted(
        font: &str,
        content_scale: i32,
        interface_scale: i32,
        font_exists: impl FnOnce(&str) -> bool,
    ) -> Self {
        let font = strip_font_size(font).and_then(|font| {
            let description = FontDescription::from_string(&font);
            let family = description.family()?;
            font_exists(&family).then_some(description)
        });
        Self {
            font,
            content_scale: persisted_scale(content_scale),
            interface_scale: persisted_scale(interface_scale),
        }
    }

    #[cfg(test)]
    pub fn font(&self) -> Option<&FontDescription> {
        self.font.as_ref()
    }

    #[cfg(test)]
    pub fn content_scale(&self) -> i32 {
        self.content_scale
    }

    #[cfg(test)]
    pub fn interface_scale(&self) -> i32 {
        self.interface_scale
    }

    pub fn css(&self) -> String {
        let interface_scale = self.interface_scale as f64 / DEFAULT_SCALE as f64;
        let content_scale = self.content_scale as f64 / self.interface_scale as f64;
        let mut font_rules = String::new();
        if let Some(font) = &self.font {
            if let Some(family) = font.family() {
                font_rules.push_str(&format!("  font-family: {};\n", css_string(&family)));
            }
            let fields = font.set_fields();
            if fields.contains(FontMask::STYLE) {
                let style = match font.style() {
                    Style::Italic => "italic",
                    Style::Oblique => "oblique",
                    _ => "normal",
                };
                font_rules.push_str(&format!("  font-style: {style};\n"));
            }
            if fields.contains(FontMask::VARIANT) {
                let variant = match font.variant() {
                    Variant::SmallCaps => "small-caps",
                    Variant::AllSmallCaps => "all-small-caps",
                    Variant::PetiteCaps => "petite-caps",
                    Variant::AllPetiteCaps => "all-petite-caps",
                    Variant::Unicase => "unicase",
                    Variant::TitleCaps => "titling-caps",
                    _ => "normal",
                };
                font_rules.push_str(&format!("  font-variant: {variant};\n"));
            }
            if fields.contains(FontMask::WEIGHT) {
                font_rules.push_str(&format!("  font-weight: {};\n", font.weight().into_glib()));
            }
            if fields.contains(FontMask::STRETCH) {
                let stretch = match font.stretch() {
                    Stretch::UltraCondensed => "ultra-condensed",
                    Stretch::ExtraCondensed => "extra-condensed",
                    Stretch::Condensed => "condensed",
                    Stretch::SemiCondensed => "semi-condensed",
                    Stretch::SemiExpanded => "semi-expanded",
                    Stretch::Expanded => "expanded",
                    Stretch::ExtraExpanded => "extra-expanded",
                    Stretch::UltraExpanded => "ultra-expanded",
                    _ => "normal",
                };
                font_rules.push_str(&format!("  font-stretch: {stretch};\n"));
            }
        }
        format!(
            ".{INTERFACE_CLASS} {{\n  font-size: {interface_scale:.4}em;\n{font_rules}}}\n\
             .{INTERFACE_CLASS} .{INTERFACE_CLASS} {{\n  font-size: 1em;\n}}\n\
             .{INTERFACE_CLASS} .{CONTENT_CLASS} {{\n  font-size: {content_scale:.4}em;\n}}\n"
        )
    }
}

pub fn clamp_scale(scale: i32) -> i32 {
    scale.clamp(MIN_SCALE, MAX_SCALE)
}

pub fn normalize_scale(scale: i32) -> i32 {
    let scale = clamp_scale(scale);
    ((scale + SCALE_STEP / 2) / SCALE_STEP * SCALE_STEP).clamp(MIN_SCALE, MAX_SCALE)
}

fn persisted_scale(scale: i32) -> i32 {
    if (MIN_SCALE..=MAX_SCALE).contains(&scale) {
        normalize_scale(scale)
    } else {
        DEFAULT_SCALE
    }
}

pub fn strip_font_size(font: &str) -> Option<String> {
    let font = font.trim();
    if font.is_empty() {
        return None;
    }
    let mut description = FontDescription::from_string(font);
    description.unset_fields(FontMask::SIZE);
    description.family()?;
    Some(description.to_str().to_string())
}

pub fn css_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\a "),
            '\r' => escaped.push_str("\\d "),
            '\0' => escaped.push_str("\\fffd "),
            value if value.is_control() => escaped.push_str(&format!("\\{:x} ", value as u32)),
            value => escaped.push(value),
        }
    }
    escaped.push('"');
    escaped
}

pub fn system_has_font(family: &str) -> bool {
    let context = gtk::Label::new(None).pango_context();
    context
        .font_map()
        .and_then(|font_map| font_map.family(family))
        .is_some()
}

pub fn install(display: &gtk::gdk::Display, preferences: &TypographyPreferences) {
    PROVIDER.with_borrow_mut(|state| {
        if state.is_none() {
            let provider = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            *state = Some(ProviderState {
                provider,
                css: String::new(),
            });
        }
    });
    apply(preferences);
}

pub fn apply(preferences: &TypographyPreferences) {
    let css = preferences.css();
    PROVIDER.with_borrow_mut(|state| {
        if let Some(state) = state.as_mut() {
            state
                .provider
                .load_from_bytes(&gtk::glib::Bytes::from_owned(css.clone()));
            state.css = css;
        }
    });
}

fn fit_dialog_to_root(dialog: &adw::Dialog, preferred_width: i32, preferred_height: i32) {
    let Some(root) = dialog.root() else { return };
    let available_width = root.width().saturating_sub(2);
    let available_height = root.height().saturating_sub(2);
    if available_width > 0 {
        dialog.set_content_width(preferred_width.min(available_width));
    }
    if available_height > 0 {
        dialog.set_content_height(preferred_height.min(available_height));
    }
}

pub fn fit_dialog_to_parent(dialog: &impl IsA<adw::Dialog>, parent: &impl IsA<gtk::Widget>) {
    let dialog = dialog.upcast_ref::<adw::Dialog>();
    let available_width = parent.width().saturating_sub(2);
    let available_height = parent.height().saturating_sub(2);
    if available_width > 0 && dialog.content_width() > available_width {
        dialog.set_content_width(available_width);
    }
    if available_height > 0 && dialog.content_height() > available_height {
        dialog.set_content_height(available_height);
    }
}

pub fn register_interface_root(widget: &impl IsA<gtk::Widget>) {
    widget.add_css_class(INTERFACE_CLASS);
    if let Some(dialog) = widget.dynamic_cast_ref::<adw::Dialog>() {
        let preferred_width = dialog.content_width();
        let preferred_height = dialog.content_height();
        dialog.connect_root_notify(move |dialog| {
            fit_dialog_to_root(dialog, preferred_width, preferred_height);
        });
        dialog.connect_parent_notify(move |dialog| {
            fit_dialog_to_root(dialog, preferred_width, preferred_height);
        });
        dialog.connect_map(move |dialog| {
            fit_dialog_to_root(dialog, preferred_width, preferred_height);
        });
        fit_dialog_to_root(dialog, preferred_width, preferred_height);
    }
}

pub fn register_content_root(widget: &impl IsA<gtk::Widget>) {
    widget.add_css_class(CONTENT_CLASS);
}

#[cfg(test)]
pub fn current_css() -> String {
    PROVIDER.with_borrow(|state| state.as_ref().map(|state| state.css.clone()).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typography_defaults_use_system_font_at_full_scale() {
        let prefs = TypographyPreferences::from_persisted("", 100, 100, |_| true);

        assert_eq!(prefs.font(), None);
        assert_eq!(prefs.content_scale(), 100);
        assert_eq!(prefs.interface_scale(), 100);
    }

    #[test]
    fn scale_clamps_to_supported_bounds_and_snaps_to_five_percent_steps() {
        assert_eq!(clamp_scale(50), 75);
        assert_eq!(clamp_scale(300), 250);
        assert_eq!(normalize_scale(76), 75);
        assert_eq!(normalize_scale(78), 80);
        assert_eq!(normalize_scale(100), 100);
    }

    #[test]
    fn invalid_persisted_scales_fall_back_without_blocking_startup() {
        let prefs = TypographyPreferences::from_persisted("Sans", -1, 900, |_| true);

        assert_eq!(prefs.content_scale(), 100);
        assert_eq!(prefs.interface_scale(), 100);
    }

    #[test]
    fn font_size_is_stripped_but_family_and_face_are_retained() {
        let font = strip_font_size("Cantarell Bold Italic 17").unwrap();
        let description = gtk::pango::FontDescription::from_string(&font);

        assert_eq!(description.family().as_deref(), Some("Cantarell"));
        assert_eq!(description.weight(), gtk::pango::Weight::Bold);
        assert_eq!(description.style(), gtk::pango::Style::Italic);
        assert!(!description.set_fields().contains(gtk::pango::FontMask::SIZE));
    }

    #[test]
    fn absolute_font_size_is_stripped_but_family_and_face_are_retained() {
        let mut source = FontDescription::from_string("Cantarell Bold Italic");
        source.set_absolute_size(17.0 * gtk::pango::SCALE as f64);

        let font = strip_font_size(&source.to_str()).unwrap();
        let description = FontDescription::from_string(&font);

        assert_eq!(description.family().as_deref(), Some("Cantarell"));
        assert_eq!(description.weight(), gtk::pango::Weight::Bold);
        assert_eq!(description.style(), gtk::pango::Style::Italic);
        assert!(!description.set_fields().contains(FontMask::SIZE));
    }

    #[test]
    fn css_retains_extended_pango_face_attributes() {
        let mut font = FontDescription::new();
        font.set_family("Cantarell");
        font.set_variant(Variant::AllSmallCaps);
        font.set_stretch(Stretch::SemiExpanded);
        let prefs = TypographyPreferences {
            font: Some(font),
            content_scale: 100,
            interface_scale: 100,
        };

        let css = prefs.css();
        assert!(css.contains("font-variant: all-small-caps;"), "{css}");
        assert!(css.contains("font-stretch: semi-expanded;"), "{css}");
    }

    #[test]
    fn css_string_escaping_handles_quotes_backslashes_controls_and_unicode() {
        assert_eq!(
            css_string("Odd \\\" Family\n日本語"),
            "\"Odd \\\\\\\" Family\\a 日本語\""
        );
    }

    #[test]
    fn missing_font_omits_override_and_uses_system_fallback() {
        let prefs = TypographyPreferences::from_persisted("Definitely Missing Bold 18", 100, 100, |family| {
            family != "Definitely Missing"
        });

        assert_eq!(prefs.font(), None);
        assert!(!prefs.css().contains("font-family"));
    }

    #[test]
    fn css_keeps_interface_and_content_scales_independent() {
        let prefs = TypographyPreferences::from_persisted("Cantarell Bold 18", 100, 125, |_| true);
        let css = prefs.css();

        assert!(css.contains(".momentum-interface {"));
        assert!(css.contains("font-size: 1.2500em;"));
        assert!(css.contains(".momentum-interface .momentum-content {"));
        assert!(css.contains("font-size: 0.8000em;"));
        assert!(css.contains("font-family: \"Cantarell\";"));
        assert!(css.contains("font-weight: 700;"));
    }

    #[test]
    fn css_is_finite_at_supported_extremes_and_default() {
        for scale in [75, 100, 250] {
            let prefs = TypographyPreferences::from_persisted("", scale, scale, |_| true);
            let css = prefs.css();
            assert!(!css.contains("NaN"));
            assert!(!css.contains("inf"));
            assert!(css.contains("font-size: 1.0000em;"));
        }
    }
}
