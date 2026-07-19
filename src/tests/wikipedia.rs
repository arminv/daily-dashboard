use super::*;
use crossterm::event::KeyModifiers;
use ratatui::{
    Terminal,
    backend::TestBackend,
};

#[test]
fn parse_search_results_extracts_title_pageid_and_snippet() {
    let json = serde_json::json!({
        "query": {
            "search": [
                {
                    "ns": 0,
                    "title": "Rust (programming language)",
                    "pageid": 12345,
                    "wordcount": 12000,
                    "snippet": "Rust is a <span class=\"searchmatch\">multi-paradigm</span> language."
                },
                {
                    "ns": 0,
                    "title": "Rust",
                    "pageid": 67890,
                    "snippet": "Rust may refer to:"
                }
            ]
        }
    });
    let results = parse_search_results(&json);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].title, "Rust (programming language)");
    assert_eq!(results[0].page_id, 12345);
    assert_eq!(results[0].wordcount, Some(12000));
    assert_eq!(
        results[0].snippet_plain,
        "Rust is a multi-paradigm language."
    );
    assert!(
        results[0]
            .page_url
            .starts_with("https://en.wikipedia.org/wiki/Rust")
    );
    assert_eq!(results[1].title, "Rust");
    assert_eq!(results[1].page_id, 67890);
    assert!(results[1].wordcount.is_none());
}

#[test]
fn parse_search_results_skips_missing_fields_and_handles_empty() {
    let json = serde_json::json!({
        "query": {
            "search": [
                { "title": "No pageid" },
                { "pageid": 1 },
                { "title": "Ok", "pageid": 2, "snippet": "hi" }
            ]
        }
    });
    let results = parse_search_results(&json);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Ok");
    assert_eq!(results[0].page_id, 2);

    let empty = serde_json::json!({ "query": { "search": [] } });
    assert!(parse_search_results(&empty).is_empty());

    let missing = serde_json::json!({ "batchcomplete": true });
    assert!(parse_search_results(&missing).is_empty());
}

#[test]
fn strip_search_html_removes_tags_and_common_entities() {
    assert_eq!(
        strip_search_html(
            "<span class=\"searchmatch\">Nelson</span> Rolihlahla &amp; friends&#039;"
        ),
        "Nelson Rolihlahla & friends'"
    );
    assert_eq!(strip_search_html("plain text"), "plain text");
    assert_eq!(strip_search_html(""), "");
}

#[test]
fn parse_extracts_query_maps_page_id_to_description_and_extract() {
    let json = serde_json::json!({
        "query": {
            "pages": [
                {
                    "pageid": 12345,
                    "title": "Rust",
                    "description": "General-purpose programming language",
                    "extract": "Rust is a systems programming language."
                },
                {
                    "pageid": 99,
                    "title": "Empty",
                    "extract": ""
                },
                {
                    "pageid": 7,
                    "title": "No description",
                    "extract": "Just an extract."
                }
            ]
        }
    });
    let extracts = parse_extracts_query(&json);
    assert_eq!(extracts.len(), 2);
    let (desc, extract) = extracts.get(&12345).expect("page 12345");
    assert_eq!(
        desc.as_deref(),
        Some("General-purpose programming language")
    );
    assert_eq!(extract, "Rust is a systems programming language.");
    let (desc7, extract7) = extracts.get(&7).expect("page 7");
    assert!(desc7.is_none());
    assert_eq!(extract7, "Just an extract.");
    assert!(!extracts.contains_key(&99));
}

#[test]
fn apply_extracts_fills_matching_results() {
    let mut results = vec![
        WikiResult {
            title: "A".into(),
            page_id: 1,
            snippet_plain: "snip".into(),
            page_url: "https://en.wikipedia.org/wiki/A".into(),
            wordcount: None,
            description: None,
            extract: Some("snip".into()),
        },
        WikiResult {
            title: "B".into(),
            page_id: 2,
            snippet_plain: String::new(),
            page_url: "https://en.wikipedia.org/wiki/B".into(),
            wordcount: None,
            description: None,
            extract: None,
        },
    ];
    let mut map = std::collections::HashMap::new();
    map.insert(1, (Some("desc".into()), "full extract".into()));
    apply_extracts(&mut results, &map);
    assert_eq!(results[0].description.as_deref(), Some("desc"));
    assert_eq!(results[0].extract.as_deref(), Some("full extract"));
    assert!(results[1].extract.is_none());
}

#[test]
fn enter_action_searches_until_query_matches_loaded_results() {
    assert_eq!(enter_action("rust", "", false), EnterAction::Search);
    assert_eq!(enter_action("rust", "rust", false), EnterAction::Search);
    assert_eq!(enter_action("rust", "rust", true), EnterAction::Open);
    assert_eq!(enter_action("rustc", "rust", true), EnterAction::Search);
    assert_eq!(enter_action("  rust  ", "rust", true), EnterAction::Open);
}

#[test]
fn is_search_activation_key_matches_slash_without_modifiers() {
    let slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
    assert!(is_search_activation_key(slash));

    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(!is_search_activation_key(esc));

    let ctrl_slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL);
    assert!(!is_search_activation_key(ctrl_slash));

    let n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
    assert!(!is_search_activation_key(n));
}

#[test]
fn draw_not_started_shows_search_hint() {
    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let client = http::shared_client().expect("client");
    let mut wiki = Wikipedia::new(client);

    terminal
        .draw(|frame| {
            wiki.draw(frame, frame.area()).expect("draw");
        })
        .expect("draw frame");

    let buffer = terminal.backend().buffer();
    let content: String = buffer
        .content
        .iter()
        .map(|c| c.symbol().to_string())
        .collect();
    assert!(
        content.contains("press / to search") || content.contains("Press / to type"),
        "expected search hint in buffer, got: {content}"
    );
}
