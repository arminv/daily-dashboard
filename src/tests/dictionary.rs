use super::*;

fn test_client() -> reqwest::Client {
    http::shared_client().expect("failed to build test HTTP client")
}

#[test]
fn parse_entry_extracts_word_phonetic_and_meanings() {
    let json = serde_json::json!([{
        "word": "hello",
        "phonetic": "/həˈloʊ/",
        "meanings": [{
            "partOfSpeech": "exclamation",
            "definitions": [
                { "definition": "Used as a greeting.", "example": "Hello there!" },
                { "definition": "Used to express a query." }
            ]
        }]
    }]);
    let entry = parse_entry(&json[0]).expect("entry should parse");
    assert_eq!(entry.word, "hello");
    assert_eq!(entry.phonetic.as_deref(), Some("/həˈloʊ/"));
    assert_eq!(entry.meanings.len(), 1);
    assert_eq!(entry.meanings[0].part_of_speech, "exclamation");
    assert_eq!(entry.meanings[0].definitions.len(), 2);
    assert_eq!(
        entry.meanings[0].definitions[0].definition,
        "Used as a greeting."
    );
    assert_eq!(
        entry.meanings[0].definitions[0].example.as_deref(),
        Some("Hello there!")
    );
    assert!(entry.meanings[0].definitions[1].example.is_none());
}

#[test]
fn parse_entry_falls_back_to_phonetics_array() {
    let json = serde_json::json!([{
        "word": "rust",
        "phonetics": [
            { "text": "/rʌst/" },
            { "audio": "ignored.mp3" }
        ],
        "meanings": [{
            "partOfSpeech": "noun",
            "definitions": [{ "definition": "A reddish-brown oxide." }]
        }]
    }]);
    let entry = parse_entry(&json[0]).expect("entry should parse");
    assert_eq!(entry.word, "rust");
    assert_eq!(entry.phonetic.as_deref(), Some("/rʌst/"));
}

#[test]
fn parse_entry_skips_meanings_without_definitions() {
    let json = serde_json::json!([{
        "word": "x",
        "meanings": [
            { "partOfSpeech": "noun", "definitions": [{ "definition": "ok" }] },
            { "partOfSpeech": "verb", "definitions": [] },
            { "partOfSpeech": "adj" }
        ]
    }]);
    let entry = parse_entry(&json[0]).expect("entry should parse");
    assert_eq!(entry.meanings.len(), 1);
    assert_eq!(entry.meanings[0].part_of_speech, "noun");
}

#[test]
fn build_definition_text_renders_word_part_of_speech_and_definition() {
    let entries = vec![DictionaryEntry {
        word: "hello".to_string(),
        phonetic: Some("/h/".to_string()),
        meanings: vec![Meaning {
            part_of_speech: "noun".to_string(),
            definitions: vec![Definition {
                definition: "a greeting".to_string(),
                example: Some("Hello!".to_string()),
            }],
        }],
    }];
    let text = build_definition_text(&entries);
    assert!(text.lines.len() > 2, "expected multiple rendered lines");
    let rendered: String = text
        .lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref().to_string())
        .collect();
    assert!(rendered.contains("hello"));
    assert!(rendered.contains("noun"));
    assert!(rendered.contains("a greeting"));
    assert!(rendered.contains("Hello!"));
}

#[tokio::test]
#[ignore = "hits the live dictionary API"]
async fn fetch_word_definition_loads_real_word() {
    let data = Arc::new(RwLock::new(DictionaryData::default()));
    fetch_word_definition(data.clone(), "hello".to_string(), test_client()).await;

    let state = data.read().unwrap();
    assert!(
        matches!(state.loading_status, LoadingStatus::Loaded),
        "expected Loaded, got {:?}",
        state.loading_status
    );
    assert!(!state.entries.is_empty(), "expected at least one entry");
    assert_eq!(state.entries[0].word, "hello");
    assert!(
        state.entries[0]
            .meanings
            .iter()
            .any(|m| !m.definitions.is_empty()),
        "expected at least one definition"
    );
}

#[tokio::test]
#[ignore = "hits the live dictionary API"]
async fn fetch_word_definition_unknown_word_errors() {
    let data = Arc::new(RwLock::new(DictionaryData::default()));
    fetch_word_definition(data.clone(), "asdfqwertyxyz".to_string(), test_client()).await;

    let state = data.read().unwrap();
    assert!(
        matches!(state.loading_status, LoadingStatus::Error(_)),
        "expected Error, got {:?}",
        state.loading_status
    );
}

#[tokio::test]
#[ignore = "hits the live dictionary API"]
async fn fetch_word_definition_samovar_has_meanings() {
    let data = Arc::new(RwLock::new(DictionaryData::default()));
    fetch_word_definition(data.clone(), "samovar".to_string(), test_client()).await;

    let state = data.read().unwrap();
    assert!(
        matches!(state.loading_status, LoadingStatus::Loaded),
        "expected Loaded, got {:?}",
        state.loading_status
    );
    let entry = &state.entries[0];
    assert_eq!(entry.word, "samovar");
    assert!(!entry.meanings.is_empty(), "meanings empty: {entry:?}");
    assert!(
        entry.meanings[0]
            .definitions
            .iter()
            .any(|d| !d.definition.is_empty()),
        "no definitions: {:?}",
        entry.meanings[0]
    );

    let text = build_definition_text(&state.entries);
    assert!(
        text.lines.len() > 2,
        "expected multiple lines, got {}",
        text.lines.len()
    );
}
