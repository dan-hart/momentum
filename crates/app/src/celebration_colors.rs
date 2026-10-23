// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Opaque native accent colors for Celebrate's label and symbolic icon.

fn luminance(rgb: [u8; 3]) -> f64 {
    let linear = rgb.map(|channel| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
}

fn contrast(first: [u8; 3], second: [u8; 3]) -> f64 {
    let (a, b) = (luminance(first), luminance(second));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn colors(accent: [u8; 3]) -> [([u8; 3], [u8; 3]); 3] {
    let ink = if contrast(accent, [0; 3]) >= contrast(accent, [255; 3]) {
        [0; 3]
    } else {
        [255; 3]
    };
    // Shade away from the selected ink so hover/press and their native color
    // transitions only increase contrast. The opaque foreground stays stable.
    let endpoint = if ink == [0; 3] { 255.0 } else { 0.0 };
    [0.0, 0.07, 0.14].map(|amount| {
        (
            accent.map(|channel| (f64::from(channel) * (1.0 - amount) + endpoint * amount).round() as u8),
            ink,
        )
    })
}

fn stylesheet(accent: [u8; 3]) -> String {
    let mut css = String::new();
    for (state, (background, foreground)) in ["", ":hover", ":active"].into_iter().zip(colors(accent)) {
        let [r, g, b] = background;
        let [ir, ig, ib] = foreground;
        css.push_str(&format!("button.celebrate-contrast{state}:not(:disabled) {{ background-image: none; background-color: rgb({r},{g},{b}); color: rgb({ir},{ig},{ib}); }}\n"));
    }
    css
}

/// Keep native pill geometry, focus/HC outlines and disabled treatment. Only this
/// button's enabled fill and inherited label/icon color are overridden.
pub fn apply(button: &gtk::Button) {
    use gtk::prelude::*;
    let manager = adw::StyleManager::default();
    let provider = gtk::CssProvider::new();
    button.add_css_class("celebrate-contrast");
    #[allow(deprecated)]
    button
        .style_context()
        .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    let update = move |manager: &adw::StyleManager| {
        let accent = manager.accent_color_rgba();
        let rgb = [accent.red(), accent.green(), accent.blue()].map(|c| (c * 255.0).round() as u8);
        provider.load_from_string(&stylesheet(rgb));
    };
    update(&manager);
    let handler = manager.connect_accent_color_rgba_notify(update);
    // No strong button reference in the manager callback; disposing the button
    // disconnects its signal and releases the provider, including after rebuilds.
    button.add_weak_ref_notify_local(move || manager.disconnect(handler));
}

#[cfg(test)]
mod tests {
    use super::*;

    // libadwaita's nine system accent backgrounds, including yellow and blue
    // whose native white foreground misses normal-text AA contrast.
    const ACCENTS: [[u8; 3]; 9] = [
        [0x35, 0x84, 0xe4],
        [0x21, 0x90, 0xa4],
        [0x3a, 0x94, 0x4a],
        [0xc8, 0x88, 0x00],
        [0xed, 0x5b, 0x00],
        [0xe6, 0x2d, 0x42],
        [0xd5, 0x61, 0x99],
        [0x91, 0x41, 0xac],
        [0x6f, 0x83, 0x96],
    ];

    #[test]
    fn celebration_colors_cover_native_accents_and_interaction_states() {
        for accent in ACCENTS {
            let states = colors(accent);
            assert_eq!(states[0].0, accent);
            assert_ne!(states[0].0, states[1].0);
            assert_ne!(states[1].0, states[2].0);
            for (background, foreground) in states {
                assert!(
                    contrast(background, foreground) >= 4.5,
                    "{accent:?}: {background:?}/{foreground:?}"
                );
            }
        }
    }

    #[test]
    fn celebration_colors_cover_luminance_boundary_and_endpoints() {
        for gray in 0..=255 {
            let states = colors([gray; 3]);
            for (background, foreground) in states {
                assert!(contrast(background, foreground) >= 4.5, "gray={gray}");
                // State transitions never interpolate between opposite ink colors.
                assert_eq!(foreground, states[0].1);
            }
        }
        assert_eq!(contrast([0; 3], [255; 3]), 21.0);
        assert_eq!(contrast([0; 3], [0; 3]), 1.0);
    }
}
