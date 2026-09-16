// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Parsing and structuring of the short pieces of text the UI shows and reads:
//! estimates, clock times, colours, relative days, repeat descriptions, quick-add syntax.
//! Nothing here produces localised prose; that stays with the toolkit.
use crate::types::*;
use sp_model::*;

/// "1h 30m", "45m", "2h" → milliseconds. Only complete duration tokens count.
pub fn parse_estimate(s: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut num = String::new();
    let mut any = false;
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c)
        } else if c == 'h' || c == 'm' {
            total += num.parse::<f64>().ok()? * if c == 'h' { 3_600_000.0 } else { 60_000.0 };
            num.clear();
            any = true;
        } else if !c.is_whitespace() || !num.is_empty() {
            return None;
        }
    }
    (any && num.is_empty() && total.is_finite()).then_some(total)
}

/// Milliseconds → "1h 30m" or "45m".
pub fn format_estimate(ms: f64) -> String {
    let m = (ms / 60000.0).round() as u64;
    if m >= 60 {
        format!("{}h {:02}m", m / 60, m % 60)
    } else {
        format!("{m}m")
    }
}

/// `14:30`, `14.30`, `1430`, `2pm`, `2:30 pm`, `9` → hour and minute.
pub fn parse_time(s: &str) -> Option<ClockTime> {
    let t = s.trim().to_ascii_lowercase();
    if t.is_empty() {
        return None;
    }
    let (body, pm) = match (t.strip_suffix("pm"), t.strip_suffix("am")) {
        (Some(b), _) => (b.trim(), Some(true)),
        (_, Some(b)) => (b.trim(), Some(false)),
        _ => (t.as_str(), None),
    };
    let (h, m): (u32, u32) = match body.split_once([':', '.', 'h']) {
        Some((h, m)) => (
            h.trim().parse().ok()?,
            if m.is_empty() { 0 } else { m.trim().parse().ok()? },
        ),
        None if body.len() == 4 && body.chars().all(|c| c.is_ascii_digit()) => {
            (body[..2].parse().ok()?, body[2..].parse().ok()?)
        }
        None => (body.parse().ok()?, 0),
    };
    let h = match (h, pm) {
        (12, Some(false)) => 0,
        (h, Some(true)) if h < 12 => h + 12,
        (h, _) => h,
    };
    (h < 24 && m < 60).then_some(ClockTime { hour: h, minute: m })
}

/// Local clock time of an epoch-millisecond timestamp.
pub fn clock_time(ms: u64) -> ClockTime {
    let (hour, minute) = time_of_ms(ms);
    ClockTime { hour, minute }
}

/// A CSS colour from the sync data as `#rrggbb`: hex forms and `rgb()`/`rgba()`. Anything
/// else is left to the toolkit's own parser.
pub fn normalize_color(color: &str) -> Option<String> {
    let c = color.trim();
    if let Some(hex) = c.strip_prefix('#') {
        let hex: String = hex.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
        return match hex.len() {
            3 | 4 => {
                let mut out = String::from("#");
                for ch in hex.chars().take(3) {
                    out.push(ch);
                    out.push(ch);
                }
                Some(out.to_lowercase())
            }
            6 | 8 => Some(format!("#{}", hex[..6].to_lowercase())),
            _ => None,
        };
    }
    let lower = c.to_ascii_lowercase();
    if let Some(inner) = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<u8> = inner
            .split([',', ' ', '/'])
            .filter(|p| !p.is_empty())
            .take(3)
            .map(|p| {
                let p = p.trim();
                if let Some(pct) = p.strip_suffix('%') {
                    pct.parse::<f64>()
                        .ok()
                        .map(|v| (v * 2.55).round().clamp(0.0, 255.0) as u8)
                } else {
                    p.parse::<f64>().ok().map(|v| v.round().clamp(0.0, 255.0) as u8)
                }
            })
            .collect::<Option<Vec<u8>>>()?;
        if parts.len() == 3 {
            return Some(format!("#{:02x}{:02x}{:02x}", parts[0], parts[1], parts[2]));
        }
    }
    None
}

/// A project's colour: `theme.primary`.
pub fn project_color(p: &Project) -> Option<String> {
    p.color().and_then(normalize_color)
}

/// A tag's colour: its `color`, else `theme.primary`.
pub fn tag_color(g: &Tag) -> Option<String> {
    g.color
        .as_deref()
        .or_else(|| g.theme.get("primary").and_then(serde_json::Value::as_str))
        .and_then(normalize_color)
}

/// Relates a `YYYY-MM-DD` day to today. Unparsable days come back as `Other`.
pub fn day_label(day: &str) -> DayLabel {
    let today = today_str();
    let (Some(target), Some(now)) = (day_number(day), day_number(&today)) else {
        return DayLabel {
            day: day.to_string(),
            relation: DayRelation::Other,
            weekday: 0,
        };
    };
    let relation = match target - now {
        0 => DayRelation::Today,
        1 => DayRelation::Tomorrow,
        -1 => DayRelation::Yesterday,
        2..=6 => DayRelation::ThisWeek,
        _ => {
            let same_year = parse_day(day).map(|(y, _, _)| y) == parse_day(&today).map(|(y, _, _)| y);
            if same_year {
                DayRelation::ThisYear
            } else {
                DayRelation::Other
            }
        }
    };
    DayLabel {
        day: day.to_string(),
        relation,
        weekday: weekday(target),
    }
}

/// The next Monday strictly after today (a Monday moves to the following Monday).
pub fn next_monday() -> String {
    let today = day_number(&today_str()).unwrap_or(0);
    let ahead = (8 - weekday(today) as i64) % 7; // weekday: 0 = Sunday … 1 = Monday
    day_str(today + if ahead == 0 { 7 } else { ahead })
}

pub fn tomorrow() -> String {
    day_str(day_number(&today_str()).unwrap_or(0) + 1)
}

/// A repeat config in plain terms, for "Repeats every Monday" and friends.
pub fn describe_repeat(c: &RepeatCfg) -> RepeatDescription {
    let n = c.repeat_every.max(1);
    let days: Vec<u32> = c
        .weekdays()
        .iter()
        .enumerate()
        .filter(|(_, on)| **on)
        .map(|(i, _)| i as u32)
        .collect();
    match c.repeat_cycle.as_str() {
        "DAILY" if n == 1 => RepeatDescription::Daily,
        "DAILY" => RepeatDescription::EveryNDays { n },
        "WEEKLY" if days.len() == 7 && n == 1 => RepeatDescription::Daily,
        "WEEKLY" if days.len() == 1 && n == 1 => RepeatDescription::EveryWeekday { weekday: days[0] },
        "WEEKLY" => RepeatDescription::Weekly { n, weekdays: days },
        "MONTHLY" => {
            let rule = if let Some((w, d)) = c.nth_weekday_anchor() {
                MonthlyRule::NthWeekday { week: w, weekday: d }
            } else if c.monthly_last_day {
                MonthlyRule::LastDay
            } else {
                match c.start_date.as_deref().and_then(parse_day) {
                    Some((_, _, d)) => MonthlyRule::DayOfMonth { day: d },
                    None => MonthlyRule::SameDay,
                }
            };
            RepeatDescription::Monthly { n, rule }
        }
        "YEARLY" => {
            let (month, day) = c
                .start_date
                .as_deref()
                .and_then(parse_day)
                .map(|(_, m, d)| (m, d))
                .unwrap_or((1, 1));
            RepeatDescription::Yearly { n, month, day }
        }
        _ => RepeatDescription::Repeats,
    }
}

/// A quick-add line taken apart: `#tag` words become tags, a trailing `1h 30m` the
/// estimate, the rest the title.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct QuickAdd {
    pub title: String,
    pub tags: Vec<String>,
    pub estimate_ms: f64,
}

pub fn parse_quick_add(text: &str) -> QuickAdd {
    let (mut words, mut tags, mut est) = (vec![], vec![], 0.0);
    for w in text.split_whitespace() {
        if let Some(tag) = w.strip_prefix('#').filter(|t| !t.is_empty()) {
            tags.push(tag.to_string());
        } else if let Some(ms) = parse_estimate(w) {
            est += ms; // "1h 30m" is two words that add up
        } else {
            words.push(w);
        }
    }
    QuickAdd {
        title: words.join(" "),
        tags,
        estimate_ms: est,
    }
}

/// The `#word` that ends at `cursor` (a character offset) in the quick-add text.
pub fn hash_word_at(text: &str, cursor: u32) -> Option<HashWord> {
    let chars: Vec<char> = text.chars().collect();
    let cursor = (cursor as usize).min(chars.len());
    let start = chars[..cursor]
        .iter()
        .rposition(|c| c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    let word: String = chars[start..cursor].iter().collect();
    word.strip_prefix('#').map(|w| HashWord {
        start: start as u32,
        prefix: w.to_string(),
    })
}

/// Replaces the `#word` at the cursor with `#name ` and puts the cursor after it.
pub fn complete_hash_word(text: &str, cursor: u32, name: &str) -> Option<CompletedText> {
    let word = hash_word_at(text, cursor)?;
    let chars: Vec<char> = text.chars().collect();
    let start = word.start as usize;
    let end = start + 1 + word.prefix.chars().count();
    let head: String = chars[..start].iter().collect();
    let tail: String = chars[end.min(chars.len())..].iter().collect();
    let text = format!("{head}#{name} {}", tail.trim_start());
    Some(CompletedText {
        cursor: (start + name.chars().count() + 2) as u32,
        text,
    })
}

/// Text from a drop or a multi-line paste, decided into tasks: a URL or a paragraph is
/// one task with the text in its notes; several short lines are several tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextTasks {
    Link { title: String, url: String },
    Lines(Vec<String>),
    Paragraph { title: String, notes: String },
    Nothing,
}

pub fn tasks_from_text(text: &str) -> TextTasks {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return TextTasks::Nothing;
    }
    let is_url = lines.len() == 1 && (lines[0].starts_with("http://") || lines[0].starts_with("https://"));
    let all_short = lines.iter().all(|l| l.len() <= 120 && !l.starts_with("http"));
    if is_url {
        let title = lines[0]
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        TextTasks::Link {
            title,
            url: lines[0].to_string(),
        }
    } else if all_short && lines.len() > 1 {
        TextTasks::Lines(lines.into_iter().map(str::to_string).collect())
    } else {
        TextTasks::Paragraph {
            title: lines[0].chars().take(120).collect(),
            notes: text.trim().to_string(),
        }
    }
}

/// First non-empty line of a note, cut to 80 characters with an ellipsis.
pub fn notes_preview(notes: &str) -> Option<String> {
    let first = notes.lines().find(|l| !l.trim().is_empty())?.trim();
    let preview: String = first.chars().take(80).collect();
    Some(if first.chars().count() > 80 {
        format!("{preview}…")
    } else {
        preview
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_parse_and_format_both_ways() {
        assert_eq!(parse_estimate("1h"), Some(3_600_000.0));
        assert_eq!(parse_estimate("1h30m"), Some(5_400_000.0));
        assert_eq!(parse_estimate("45m"), Some(2_700_000.0));
        assert_eq!(parse_estimate("2h 15m"), Some(8_100_000.0));
        assert!(parse_estimate("soon").is_none() && parse_estimate("").is_none() && parse_estimate("h").is_none());
        assert_eq!(format_estimate(5_400_000.0), "1h 30m");
        assert_eq!(format_estimate(3_600_000.0), "1h 00m");
        assert_eq!(format_estimate(600_000.0), "10m");
        assert_eq!(parse_estimate(&format_estimate(8_100_000.0)), Some(8_100_000.0));
    }
    #[test]
    fn times() {
        let t = |h, m| Some(ClockTime { hour: h, minute: m });
        assert_eq!(parse_time("14:30"), t(14, 30));
        assert_eq!(parse_time(" 9 "), t(9, 0));
        assert_eq!(parse_time("2pm"), t(14, 0));
        assert_eq!(parse_time("2:30 PM"), t(14, 30));
        assert_eq!(parse_time("12am"), t(0, 0));
        assert_eq!(parse_time("12:15pm"), t(12, 15));
        assert_eq!(parse_time("0930"), t(9, 30));
        assert_eq!(parse_time("14.05"), t(14, 5));
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time(""), None);
    }
    #[test]
    fn colours() {
        assert_eq!(normalize_color("#ff8800").as_deref(), Some("#ff8800"));
        assert_eq!(normalize_color("#FF8800AA").as_deref(), Some("#ff8800"));
        assert_eq!(normalize_color("#f80").as_deref(), Some("#ff8800"));
        assert_eq!(normalize_color("rgb(255, 0, 0)").as_deref(), Some("#ff0000"));
        assert_eq!(normalize_color("rgba(0,128,255,0.5)").as_deref(), Some("#0080ff"));
        assert!(normalize_color("not-a-colour").is_none());
        let mut g = Tag::new("g");
        assert!(tag_color(&g).is_some(), "new tags carry a theme colour");
        g.color = Some("#123456".into());
        assert_eq!(tag_color(&g).as_deref(), Some("#123456"));
    }
    #[test]
    fn relative_days() {
        let n = day_number(&today_str()).unwrap();
        assert_eq!(day_label(&today_str()).relation, DayRelation::Today);
        assert_eq!(day_label(&day_str(n + 1)).relation, DayRelation::Tomorrow);
        assert_eq!(day_label(&day_str(n - 1)).relation, DayRelation::Yesterday);
        let l = day_label(&day_str(n + 3));
        assert_eq!(l.relation, DayRelation::ThisWeek);
        assert_eq!(l.weekday, weekday(n + 3));
        assert!(matches!(
            day_label(&day_str(n + 400)).relation,
            DayRelation::Other | DayRelation::ThisYear
        ));
        assert_eq!(day_label("not a day").relation, DayRelation::Other);
        let monday = day_number(&next_monday()).unwrap();
        assert_eq!(weekday(monday), 1);
        assert!(monday > n && monday <= n + 7);
    }
    #[test]
    fn repeat_descriptions() {
        let mut c = RepeatCfg {
            repeat_cycle: "DAILY".into(),
            repeat_every: 1,
            ..Default::default()
        };
        assert_eq!(describe_repeat(&c), RepeatDescription::Daily);
        c.repeat_every = 3;
        assert_eq!(describe_repeat(&c), RepeatDescription::EveryNDays { n: 3 });
        c.repeat_cycle = "WEEKLY".into();
        c.repeat_every = 1;
        c.monday = true;
        assert_eq!(describe_repeat(&c), RepeatDescription::EveryWeekday { weekday: 1 });
        c.wednesday = true;
        assert_eq!(
            describe_repeat(&c),
            RepeatDescription::Weekly {
                n: 1,
                weekdays: vec![1, 3]
            }
        );
        c.repeat_cycle = "MONTHLY".into();
        c.start_date = Some("2026-03-05".into());
        assert_eq!(
            describe_repeat(&c),
            RepeatDescription::Monthly {
                n: 1,
                rule: MonthlyRule::DayOfMonth { day: 5 }
            }
        );
        c.monthly_week_of_month = Some(2);
        c.monthly_weekday = Some(2);
        assert_eq!(
            describe_repeat(&c),
            RepeatDescription::Monthly {
                n: 1,
                rule: MonthlyRule::NthWeekday { week: 2, weekday: 2 }
            }
        );
        c.repeat_cycle = "YEARLY".into();
        assert_eq!(
            describe_repeat(&c),
            RepeatDescription::Yearly { n: 1, month: 3, day: 5 }
        );
    }
    #[test]
    fn quick_add_syntax() {
        let q = parse_quick_add("Fix the bug #work #urgent 1h 30m");
        assert_eq!(q.title, "Fix the bug");
        assert_eq!(q.tags, vec!["work", "urgent"]);
        assert_eq!(q.estimate_ms, 5_400_000.0);
        assert_eq!(parse_quick_add("   ").title, "");
    }
    #[test]
    fn hash_completion() {
        let w = hash_word_at("Write docs #gn", 14).unwrap();
        assert_eq!((w.start, w.prefix.as_str()), (11, "gn"));
        assert!(hash_word_at("Write docs", 10).is_none());
        let c = complete_hash_word("Write docs #gn tomorrow", 14, "gnome").unwrap();
        assert_eq!(c.text, "Write docs #gnome tomorrow");
        assert_eq!(c.cursor, 18);
        let c = complete_hash_word("Wo #", 4, "work").unwrap();
        assert_eq!(c.text, "Wo #work ");
    }
    #[test]
    fn text_to_tasks() {
        assert_eq!(
            tasks_from_text("first thing\nsecond thing\n\n"),
            TextTasks::Lines(vec!["first thing".into(), "second thing".into()])
        );
        assert_eq!(
            tasks_from_text("https://example.org/page/"),
            TextTasks::Link {
                title: "example.org/page".into(),
                url: "https://example.org/page/".into()
            }
        );
        let long = "A rather long paragraph ".repeat(10);
        match tasks_from_text(&long) {
            TextTasks::Paragraph { title, notes } => {
                assert!(title.chars().count() <= 120 && notes.len() > 120)
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(tasks_from_text("  \n "), TextTasks::Nothing);
        assert_eq!(notes_preview("\n  hello\nworld").as_deref(), Some("hello"));
        assert!(notes_preview("   ").is_none());
    }
}

#[cfg(test)]
mod estimate_validation_tests {
    use super::*;

    #[test]
    fn short_syntax_does_not_consume_malformed_duration_words() {
        for word in ["1hour", "1h30", "1huh", "-1h", "1h!", "NaNm"] {
            assert_eq!(parse_estimate(word), None, "{word}");
            assert_eq!(parse_quick_add(&format!("Read {word}")).title, format!("Read {word}"));
        }
        assert_eq!(parse_estimate("1h 30m"), Some(5_400_000.0));
        assert_eq!(parse_estimate("1h30m"), Some(5_400_000.0));
        assert_eq!(parse_estimate(".5h"), Some(1_800_000.0));
    }
}
