use crate::{
    action::Action,
    app::LoadingStatus,
    components::Component,
    http,
    theme,
    tui::Event,
};
use chrono::Local;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{
        Constraint,
        Direction,
        Layout,
        Rect,
    },
    style::{
        Color,
        Modifier,
        Style,
    },
    text::{
        Line,
        Span,
        Text,
    },
    widgets::{
        Block,
        Paragraph,
        Wrap,
    },
};
use ratatui_textarea::TextArea;
use std::sync::{
    Arc,
    RwLock,
};
use tracing::{
    error,
    info,
};

const DICTIONARY_API_URL: &str = "https://api.dictionaryapi.dev/api/v2/entries/en";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Clone, Debug, Default)]
struct Definition {
    definition: String,
    example: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct Meaning {
    part_of_speech: String,
    definitions: Vec<Definition>,
}

#[derive(Clone, Debug, Default)]
struct DictionaryEntry {
    word: String,
    phonetic: Option<String>,
    meanings: Vec<Meaning>,
}

/// Send-able state shared with the background fetch task.
#[derive(Clone, Debug, Default)]
pub struct DictionaryData {
    loading_status: LoadingStatus,
    search_word: String,
    entries: Vec<DictionaryEntry>,
    last_updated_at: Option<chrono::DateTime<Local>>,
}

pub struct Dictionary {
    data: Arc<RwLock<DictionaryData>>,
    // `TextArea` is not `Send`, so it lives outside the shared state.
    input: TextArea<'static>,
    input_mode: InputMode,
    client: reqwest::Client,
}

impl Dictionary {
    pub fn new(client: reqwest::Client) -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default().green());
        input.set_cursor_style(Style::default().bg(Color::Red));
        input.set_placeholder_text("Lookup a word in dictionary...");

        Self {
            data: Arc::new(RwLock::new(DictionaryData::default())),
            input,
            input_mode: InputMode::Normal,
            client,
        }
    }

    fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing;
    }

    fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    fn submit_search(&mut self) {
        let word = self.input.lines().join("").trim().to_string();
        info!("Dictionary: submit_search called with word={:?}", word);
        if word.is_empty() {
            info!("Dictionary: submit_search skipped — empty input");
            return;
        }

        {
            let mut data = self.data.write().unwrap();
            data.search_word = word.clone();
            data.entries.clear();
            data.loading_status = LoadingStatus::Loading;
        }

        let data = self.data.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            fetch_word_definition(data, word, client).await;
        });
    }

    fn render_input(&mut self, frame: &mut Frame, area: Rect) {
        let (title, title_style) = match self.input_mode {
            InputMode::Normal => (
                "🔍 Dictionary — press Esc to type a word",
                Style::default().fg(theme::ACCENT),
            ),
            InputMode::Editing => (
                "🔍 Dictionary — Enter to search · Esc to done",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        };
        self.input.set_block(
            Block::default()
                .title(title)
                .style(title_style)
                .borders(ratatui::widgets::Borders::ALL),
        );
        frame.render_widget(&self.input, area);
    }

    fn render_definitions(&self, frame: &mut Frame, area: Rect) {
        let data = self.data.read().unwrap();

        let block = theme::panel_block("📖 Definition");

        match &data.loading_status {
            LoadingStatus::NotStarted => {
                let paragraph =
                    Paragraph::new("Press Esc to type a word above, then Enter to look it up.")
                        .block(block)
                        .style(Style::default().fg(theme::HINT))
                        .wrap(Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
            LoadingStatus::Loading => {
                let paragraph = Paragraph::new(format!("Looking up \"{}\"...", data.search_word))
                    .block(block)
                    .style(Style::default().fg(theme::LOADING))
                    .wrap(Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
            LoadingStatus::Error(error) => {
                let lines = vec![
                    Line::from(Span::styled(
                        format!("No results for \"{}\"", data.search_word),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        error.clone(),
                        Style::default().fg(Color::LightRed),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Try another word.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
            LoadingStatus::Loaded => {
                let text = build_definition_text(&data.entries);
                let paragraph = Paragraph::new(text)
                    .block(block)
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
        }
    }
}

async fn fetch_word_definition(
    data: Arc<RwLock<DictionaryData>>,
    word: String,
    client: reqwest::Client,
) {
    {
        let mut state = data.write().unwrap();
        state.loading_status = LoadingStatus::Loading;
    }

    let encoded_word = word.replace(' ', "%20");
    let api_url = format!("{DICTIONARY_API_URL}/{encoded_word}");
    info!("Dictionary: fetching {api_url}");

    let json: serde_json::Value = match http::get_json(&client, &api_url).await {
        Ok(json) => json,
        Err(e) => {
            let error_msg = format!("{e}");
            error!("Dictionary: {error_msg}");
            let mut state = data.write().unwrap();
            state.loading_status = LoadingStatus::Error(error_msg);
            return;
        }
    };
    let entries_array = match json.as_array() {
        Some(array) if !array.is_empty() => array,
        _ => {
            let resolution = json
                .get("resolution")
                .and_then(|v| v.as_str())
                .unwrap_or("No definitions found");
            let mut state = data.write().unwrap();
            state.loading_status = LoadingStatus::Error(resolution.to_string());
            return;
        }
    };

    let entries: Vec<DictionaryEntry> = entries_array.iter().filter_map(parse_entry).collect();
    if entries.is_empty() {
        let mut state = data.write().unwrap();
        state.loading_status = LoadingStatus::Error("No definitions found".to_string());
        return;
    }

    let mut state = data.write().unwrap();
    state.entries = entries;
    state.last_updated_at = Some(Local::now());
    state.loading_status = LoadingStatus::Loaded;
    info!("Dictionary: loaded definition for {:?}", state.search_word);
}

fn parse_entry(entry: &serde_json::Value) -> Option<DictionaryEntry> {
    let word = entry.get("word")?.as_str()?.to_string();

    let phonetic = entry
        .get("phonetic")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            entry.get("phonetics").and_then(|phonetics| {
                phonetics.as_array().and_then(|array| {
                    array
                        .iter()
                        .filter_map(|p| p.get("text")?.as_str().map(|s| s.to_string()))
                        .next()
                })
            })
        });

    let meanings = entry
        .get("meanings")
        .and_then(|v| v.as_array())
        .map(|array| {
            array
                .iter()
                .filter_map(|meaning| {
                    let part_of_speech = meaning
                        .get("partOfSpeech")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let definitions: Vec<Definition> = meaning
                        .get("definitions")
                        .and_then(|v| v.as_array())
                        .map(|defs| {
                            defs.iter()
                                .filter_map(|d| {
                                    let definition = d
                                        .get("definition")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if definition.is_empty() {
                                        return None;
                                    }
                                    let example = d
                                        .get("example")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    Some(Definition {
                                        definition,
                                        example,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    if definitions.is_empty() {
                        return None;
                    }

                    Some(Meaning {
                        part_of_speech,
                        definitions,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(DictionaryEntry {
        word,
        phonetic,
        meanings,
    })
}

fn build_definition_text(entries: &[DictionaryEntry]) -> Text<'static> {
    let mut lines: Vec<Line> = Vec::new();

    for entry in entries {
        let mut header_spans = vec![Span::styled(
            entry.word.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )];
        if let Some(phonetic) = &entry.phonetic {
            header_spans.push(Span::raw(" "));
            header_spans.push(Span::styled(
                phonetic.clone(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::ITALIC),
            ));
        }
        lines.push(Line::from(header_spans));
        lines.push(Line::from(""));

        for meaning in &entry.meanings {
            lines.push(Line::from(Span::styled(
                meaning.part_of_speech.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));

            for definition in &meaning.definitions {
                let def_line = Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        definition.definition.clone(),
                        Style::default().fg(Color::White),
                    ),
                ]);
                lines.push(def_line);

                if let Some(example) = &definition.example {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            example.clone(),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }
    }

    Text::from(lines)
}

impl Component for Dictionary {
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        let Some(Event::Key(key)) = event else {
            return Ok(None);
        };

        // While editing we consume every keypress so sibling components (e.g.
        // News, where `Enter` opens an article and `i`/`j` scroll) don't also
        // react to it. Returning `Some(Action::Render)` tells the Dashboard to
        // stop propagating the event to the remaining components.
        match self.input_mode {
            InputMode::Normal => {
                if key.code == KeyCode::Esc {
                    self.start_editing();
                    return Ok(Some(Action::Render));
                }
                Ok(None)
            }
            InputMode::Editing => {
                match key.code {
                    KeyCode::Esc => self.stop_editing(),
                    KeyCode::Enter => self.submit_search(),
                    _ => {
                        self.input.input(key);
                    }
                };
                Ok(Some(Action::Render))
            }
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if action == Action::Tick {
            let cursor_color = if self.input_mode == InputMode::Editing {
                Color::Red
            } else {
                Color::Blue
            };
            self.input
                .set_cursor_style(Style::default().bg(cursor_color));
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        self.render_input(frame, chunks[0]);
        self.render_definitions(frame, chunks[1]);
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/dictionary.rs"]
mod tests;
