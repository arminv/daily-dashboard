use super::*;

#[tokio::test]
#[ignore = "hits the live dictionary API"]
async fn fetch_word_definition_loads_real_word() {
    let data = Arc::new(RwLock::new(DictionaryData::default()));
    fetch_word_definition(data.clone(), "hello".to_string()).await;

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
    fetch_word_definition(data.clone(), "asdfqwertyxyz".to_string()).await;

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
    fetch_word_definition(data.clone(), "samovar".to_string()).await;

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
