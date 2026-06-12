use super::Component;
use crate::action::Action;
use crate::app::LoadingStatus;
use chrono::{Datelike, Local};
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::sync::{Arc, RwLock};
use tracing::error;

const QUOTE_API_URL: &str = "https://zenquotes.io/api/today";
const DICTIONARY_API_URL: &str = "https://api.dictionaryapi.dev/api/v2/entries/en";

const WORDS: &[&str] = &[
    "serendipity",
    "ephemeral",
    "resilience",
    "eloquent",
    "luminous",
    "wanderlust",
    "nostalgia",
    "petrichor",
    "sonder",
    "equanimity",
    "mellifluous",
    "limerence",
    "effervescent",
    "halcyon",
    "ebullient",
    "lucid",
    "verdant",
    "ethereal",
    "surreptitious",
    "perspicacious",
    "magnanimous",
    "ubiquitous",
    "quintessential",
    "paradigm",
    "catalyst",
    "zenith",
    "nadir",
    "epiphany",
    "catharsis",
    "diligent",
    "pragmatic",
    "tenacious",
    "altruistic",
    "prudent",
    "voracious",
    "meticulous",
    "ingenious",
    "resplendent",
    "incandescent",
    "iridescent",
    "cacophony",
    "euphony",
    "labyrinth",
    "mirage",
    "odyssey",
    "renaissance",
    "solitude",
    "tranquil",
    "vivid",
    "whimsical",
    "yearning",
    "zealous",
    "ambivalent",
    "benevolent",
    "capricious",
    "dauntless",
    "enigmatic",
    "facetious",
    "gregarious",
    "haphazard",
    "impeccable",
    "jubilant",
    "kinetic",
    "languid",
    "mercurial",
    "nonchalant",
    "obstinate",
    "pernicious",
    "quixotic",
    "recalcitrant",
    "sagacious",
    "taciturn",
    "unctuous",
    "vociferous",
    "wistful",
    "xenial",
    "yielding",
    "zephyr",
    "arduous",
    "belligerent",
    "candid",
    "deft",
    "empirical",
    "fallible",
    "garrulous",
    "heuristic",
    "immutable",
    "jovial",
    "kaleidoscope",
    "laconic",
    "malleable",
    "neophyte",
    "ostensible",
    "plausible",
    "querulous",
    "rudimentary",
    "salient",
    "tangible",
    "untenable",
    "verbose",
    "wary",
    "abstruse",
    "bucolic",
    "cogent",
    "didactic",
    "erudite",
    "fastidious",
    "germane",
    "harbinger",
    "iconoclast",
    "juxtapose",
    "kismet",
    "lethargic",
    "maverick",
    "nefarious",
    "obfuscate",
    "perfunctory",
    "quagmire",
    "reverie",
    "sycophant",
    "truculent",
    "vicarious",
    "wane",
    "yoke",
    "zeppelin",
];

#[derive(Clone, Debug, Default)]
pub struct InspirationState {
    pub loading_status: LoadingStatus,
    pub quote_text: String,
    pub quote_author: String,
    pub word: String,
    pub word_part_of_speech: String,
    pub word_definition: String,
}

#[derive(Clone, Debug)]
pub struct Inspiration {
    state: Arc<RwLock<InspirationState>>,
}

impl Inspiration {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(InspirationState::default())),
        }
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    fn word_for_today() -> &'static str {
        let today = Local::now().date_naive();
        let day_index = today.ordinal0() as usize;
        WORDS[day_index % WORDS.len()]
    }

    async fn fetch_daily_content(&self) {
        self.set_loading_state(LoadingStatus::Loading);

        let (quote_text, quote_author) = match self.fetch_quote().await {
            Ok(quote) => quote,
            Err(e) => {
                let error_msg = format!("Failed to fetch quote: {e}");
                error!("Inspiration (Quote): {error_msg}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let word = Self::word_for_today();
        let (word_part_of_speech, word_definition) = match self.fetch_word_definition(word).await {
            Ok(definition) => definition,
            Err(e) => {
                let error_msg = format!("Failed to fetch word definition: {e}");
                error!("Inspiration (Word): {error_msg}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let mut state = self.state.write().unwrap();
        state.quote_text = quote_text;
        state.quote_author = quote_author;
        state.word = word.to_string();
        state.word_part_of_speech = word_part_of_speech;
        state.word_definition = word_definition;
        state.loading_status = LoadingStatus::Loaded;
    }

    async fn fetch_quote(&self) -> Result<(String, String), String> {
        let response = reqwest::get(QUOTE_API_URL)
            .await
            .map_err(|e| format!("API request failed: {e:?}"))?;

        if !response.status().is_success() {
            return Err(format!("API returned error status: {}", response.status()));
        }

        let body_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e:?}"))?;

        let json: serde_json::Value =
            serde_json::from_str(&body_text).map_err(|e| format!("Failed to parse JSON: {e:?}"))?;

        let entry = json
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| "Unexpected quote response format".to_string())?;

        let quote_text = entry
            .get("q")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing quote text".to_string())?
            .to_string();

        let quote_author = entry
            .get("a")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing quote author".to_string())?
            .to_string();

        Ok((quote_text, quote_author))
    }

    async fn fetch_word_definition(&self, word: &str) -> Result<(String, String), String> {
        let url = format!("{DICTIONARY_API_URL}/{word}");
        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("API request failed: {e:?}"))?;

        if !response.status().is_success() {
            return Err(format!("API returned error status: {}", response.status()));
        }

        let body_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e:?}"))?;

        let json: serde_json::Value =
            serde_json::from_str(&body_text).map_err(|e| format!("Failed to parse JSON: {e:?}"))?;

        let entry = json
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| "Unexpected dictionary response format".to_string())?;

        let meaning = entry
            .get("meanings")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| "Missing word meanings".to_string())?;

        let part_of_speech = meaning
            .get("partOfSpeech")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let definition = meaning
            .get("definitions")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|def| def.get("definition"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing word definition".to_string())?
            .to_string();

        Ok((part_of_speech, definition))
    }
}

impl Component for Inspiration {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if action == Action::Tick {
            let should_fetch = {
                let state = self.state.read().unwrap();
                let is_initial_load = matches!(
                    state.loading_status,
                    LoadingStatus::NotStarted | LoadingStatus::Error(_)
                );
                is_initial_load
            };

            if should_fetch {
                let this = self.clone();
                tokio::spawn(async move {
                    this.fetch_daily_content().await;
                });
            }
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let state = self.state.read().unwrap();

        let inspiration_area = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width,
            height: area.height,
        };

        match &state.loading_status {
            LoadingStatus::NotStarted => {
                let block = Block::default()
                    .title("Daily Inspiration")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(block, inspiration_area);
            }
            LoadingStatus::Loading => {
                let block = Block::default()
                    .title("Daily Inspiration - Loading...")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(block, inspiration_area);
            }
            LoadingStatus::Error(error) => {
                let block = Block::default()
                    .title(format!("Daily Inspiration - Error: {error}"))
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Red));
                frame.render_widget(block, inspiration_area);
            }
            LoadingStatus::Loaded => {
                let quote_line = Line::from(vec![
                    Span::styled("❝", Style::default().fg(Color::Cyan)),
                    Span::styled(state.quote_text.clone(), Style::default().fg(Color::White)),
                    Span::styled("❞", Style::default().fg(Color::Cyan)),
                ]);

                let author_line = Line::from(vec![
                    Span::styled("— ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        state.quote_author.clone(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]);

                let word_line = Line::from(vec![
                    Span::styled("Daily Word: ", Style::default().fg(Color::Magenta)),
                    Span::styled(
                        state.word.clone(),
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        if state.word_part_of_speech.is_empty() {
                            " — ".to_string()
                        } else {
                            format!(" ({}) — ", state.word_part_of_speech)
                        },
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        state.word_definition.clone(),
                        Style::default().fg(Color::Gray),
                    ),
                ]);

                let paragraph = Paragraph::new(vec![quote_line, author_line, word_line])
                    .style(Style::new().cyan())
                    .block(Block::default().title("Daily Inspiration"))
                    .wrap(Wrap { trim: true });

                frame.render_widget(paragraph, inspiration_area);
            }
        }

        Ok(())
    }
}
