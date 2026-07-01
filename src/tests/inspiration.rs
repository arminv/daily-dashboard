use super::*;
use ratatui::{Terminal, backend::TestBackend};

/// Render `widget` into a fresh test buffer and return the joined cell symbols
/// so tests can assert on the visible text without pinning exact spacing.
fn render(widget: &mut Inspiration, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("failed to build test terminal");
    terminal
        .draw(|f| widget.draw(f, f.area()).expect("draw failed"))
        .expect("terminal draw failed");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect()
}

#[test]
fn inspiration_loaded_renders_quote_and_author() {
    let mut widget = Inspiration::new(reqwest::Client::default());
    {
        let mut state = widget.state.write().unwrap();
        state.quote_text = "Stay hungry, stay foolish.".to_string();
        state.quote_author = "Steve".to_string();
        state.loading_status = LoadingStatus::Loaded;
    }

    let rendered = render(&mut widget, 44, 6);
    assert!(
        rendered.contains("Stay hungry, stay foolish."),
        "quote text should be rendered: {rendered:?}"
    );
    assert!(
        rendered.contains("Steve"),
        "author should be rendered: {rendered:?}"
    );
    assert!(
        rendered.contains("Daily Inspiration"),
        "panel title should be rendered: {rendered:?}"
    );
}

#[test]
fn inspiration_error_renders_message() {
    let mut widget = Inspiration::new(reqwest::Client::default());
    widget.state.write().unwrap().loading_status = LoadingStatus::Error("boom".to_string());

    let rendered = render(&mut widget, 48, 5);
    assert!(
        rendered.contains("Couldn't load today's quote: boom"),
        "error message should be rendered: {rendered:?}"
    );
    assert!(
        rendered.contains("Error"),
        "error title should be rendered: {rendered:?}"
    );
}
