// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use crate::{listing, text::title_from_url, InboxCleanupChange};
use momentum_model::{AppData, Tag, Task, INBOX_PROJECT_ID, TODAY_TAG_ID};
use serde_json::{json, Map, Value};
use std::collections::HashSet;

pub(crate) fn eligible(task: &Task, state: &AppData) -> bool {
    let mut current = task;
    let mut seen = HashSet::new();
    loop {
        if current.is_done || !seen.insert(&current.id) {
            return false;
        }
        let Some(parent) = &current.parent_id else {
            return current.project_id == INBOX_PROJECT_ID;
        };
        let Some(parent) = state.task.entities.get(parent) else {
            return false;
        };
        current = parent;
    }
}

pub(crate) fn eligible_tag(tag: &Tag) -> bool {
    tag.id != TODAY_TAG_ID && listing::slot_from_names([tag.title.as_str()]).is_none()
}

fn revision(task: &Task) -> String {
    json!([task.title, task.notes, task.tag_ids]).to_string()
}

fn link_prefix(text: &str) -> bool {
    ["https://", "http://", "www."].into_iter().any(|prefix| {
        text.get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    })
}

/// Return text with web links removed, plus the original link strings in first-seen order.
fn extract_links(text: &str) -> (String, Vec<String>) {
    let mut remaining = text;
    let mut title = String::new();
    let mut links = Vec::new();
    while let Some(start) = remaining.char_indices().find_map(|(index, _)| {
        let boundary = remaining[..index]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || "([{<:—–,;\"'“‘".contains(c));
        (boundary && link_prefix(&remaining[index..])).then_some(index)
    }) {
        let before = &remaining[..start];
        let candidate = &remaining[start..];
        let end = candidate
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '”' | '’'))
            .unwrap_or(candidate.len());
        let end = candidate[..end]
            .char_indices()
            .find_map(|(index, c)| (matches!(c, ',' | ';') && link_prefix(&candidate[index + 1..])).then_some(index))
            .unwrap_or(end);
        let mut link = candidate[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
            while link.ends_with(close) && link.matches(close).count() > link.matches(open).count() {
                link = link.strip_suffix(close).unwrap();
            }
        }
        let authority = link
            .split_once("://")
            .map_or(link, |(_, rest)| rest)
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("");
        if !authority.trim_matches('.').chars().any(char::is_alphanumeric) || authority.eq_ignore_ascii_case("www") {
            title.push_str(&remaining[..start + end]);
            remaining = &remaining[start + end..];
            continue;
        }

        let suffix = &candidate[link.len()..];
        if let Some((prefix, rest)) = [
            ('(', ')'),
            ('[', ']'),
            ('{', '}'),
            ('<', '>'),
            ('"', '"'),
            ('\'', '\''),
            ('“', '”'),
            ('‘', '’'),
        ]
        .into_iter()
        .find_map(|(open, close)| Some((before.strip_suffix(open)?, suffix.strip_prefix(close)?)))
        {
            title.push_str(prefix);
            remaining = rest;
        } else {
            title.push_str(before);
            remaining = suffix;
        }
        if !links.iter().any(|existing| existing == link) {
            links.push(link.to_string());
        }
    }
    title.push_str(remaining);
    (title, links)
}

fn tidy_title(text: &str) -> String {
    let mut result = String::new();
    for c in text.split_whitespace().collect::<Vec<_>>().join(" ").chars() {
        if matches!(c, '!' | '?' | ',' | ';' | ':') {
            if result.ends_with(c) {
                continue;
            }
            if result.ends_with(' ') {
                result.pop();
            }
        }
        result.push(c);
    }
    let trimmed = result.trim_matches(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '|' | '–' | '—'));
    let Some(first) = trimmed.chars().next() else {
        return String::new();
    };
    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    let mut letters = first_word.chars().filter(|c| c.is_alphabetic());
    if first.is_lowercase()
        && !letters
            .nth(1)
            .is_some_and(|_| first_word.chars().any(char::is_uppercase))
    {
        format!("{}{}", first.to_uppercase(), &trimmed[first.len_utf8()..])
    } else {
        trimmed.to_string()
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

pub(crate) fn preview(task: &Task, tags: &[(&Tag, Vec<String>)]) -> InboxCleanupChange {
    let (text, links) = extract_links(&task.title);
    let tidy = tidy_title(&text);
    let title = if tidy.chars().any(char::is_alphanumeric) || links.is_empty() {
        if tidy.is_empty() {
            task.title.clone()
        } else {
            tidy
        }
    } else {
        links
            .first()
            .and_then(|link| {
                let normalized = link
                    .split_once("://")
                    .map_or_else(|| format!("https://{link}"), |(_, rest)| format!("https://{rest}"));
                title_from_url(&normalized)
            })
            .unwrap_or_else(|| task.title.clone())
    };
    let title_words = words(&title);
    let tag_ids = tags
        .iter()
        .filter(|(tag, phrase)| {
            !phrase.is_empty()
                && !task.tag_ids.contains(&tag.id)
                && title_words.windows(phrase.len()).any(|part| part == phrase)
        })
        .map(|(tag, _)| tag.id.clone())
        .collect();
    InboxCleanupChange {
        task_id: task.id.clone(),
        source_revision: revision(task),
        original_title: task.title.clone(),
        title,
        links,
        tag_ids,
    }
}

pub(crate) fn tags(state: &AppData) -> Vec<(&Tag, Vec<String>)> {
    state
        .tag
        .iter()
        .filter(|tag| eligible_tag(tag))
        .map(|tag| (tag, words(&tag.title)))
        .collect()
}

pub(crate) fn patches(
    task: &Task,
    state: &AppData,
    proposal: InboxCleanupChange,
) -> Option<(Map<String, Value>, Map<String, Value>)> {
    if !eligible(task, state) || proposal.original_title != task.title || proposal.source_revision != revision(task) {
        return None;
    }
    let (_, links) = extract_links(&task.title);
    let title = proposal.title.trim();
    if links != proposal.links
        || (title != task.title && (title.is_empty() || title.chars().any(char::is_control)))
        || (title != task.title && !extract_links(title).1.is_empty())
    {
        return None;
    }
    let mut changes = Map::new();
    let mut undo = Map::new();
    if title != task.title {
        changes.insert("title".into(), json!(title));
        undo.insert("title".into(), json!(task.title));
    }
    if !links.is_empty() {
        let mut notes = task.notes.clone().unwrap_or_default();
        let existing = extract_links(&notes).1;
        let additions: Vec<_> = links.iter().filter(|link| !existing.contains(link)).cloned().collect();
        if !additions.is_empty() {
            if !notes.is_empty() {
                notes.push_str(if notes.ends_with("\n\n") {
                    ""
                } else if notes.ends_with('\n') {
                    "\n"
                } else {
                    "\n\n"
                });
            }
            notes.push_str(&additions.join("\n"));
            changes.insert("notes".into(), json!(notes));
            undo.insert("notes".into(), json!(task.notes));
        }
    }
    let mut tag_ids = task.tag_ids.clone();
    for id in proposal.tag_ids {
        if !state.tag.entities.get(&id).is_some_and(eligible_tag) {
            return None;
        }
        if !tag_ids.contains(&id) {
            tag_ids.push(id);
        }
    }
    if tag_ids != task.tag_ids {
        changes.insert("tagIds".into(), json!(tag_ids));
        undo.insert("tagIds".into(), json!(task.tag_ids));
    }
    Some((changes, undo))
}
