use crate::{action::Action, components::Component, tui::Event};
use color_eyre::eyre::ErrReport;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
};
use ratatui_textarea::TextArea;
use std::sync::{Arc, RwLock};
use tracing::info;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Clone, Debug, Default)]
pub struct DictionaryState {
    // TODO:
    // loading_status: LoadingStatus,
    // search_word: String,
    // search_word_definition: String,
    // last_updated_at: Option<chrono::DateTime<Local>>,
    input: TextArea<'static>,
    /// Current input mode
    input_mode: InputMode,
}

#[derive(Clone, Debug)]
pub struct Dictionary {
    state: Arc<RwLock<DictionaryState>>,
}

impl Dictionary {
    pub fn new() -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default().blue());
        input.set_cursor_style(Style::default().bg(Color::Red));
        input.set_placeholder_text("Enter a valid float (e.g. 1.56)");

        Self {
            state: Arc::new(RwLock::new(DictionaryState {
                input,
                ..Default::default()
            })),
        }
    }

    // TODO:
    // fn set_loading_state(&self, status: LoadingStatus) {
    //     let mut state = self.state.write().unwrap();
    //     state.loading_status = status;
    // }

    fn start_editing(&mut self) {
        self.state.write().unwrap().input_mode = InputMode::Editing;
    }

    fn stop_editing(&mut self) {
        self.state.write().unwrap().input_mode = InputMode::Normal;
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let state_read = self.state.read().unwrap();
        let input = state_read.input.clone();
        frame.render_widget(&input, area);
    }
}

impl Component for Dictionary {
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        let input_mode = self.state.read().unwrap().input_mode;
        match input_mode {
            InputMode::Normal => {
                if let Some(Event::Key(key)) = event
                    && key.code == KeyCode::Esc
                {
                    self.start_editing()
                }
            }
            InputMode::Editing => {
                // TODO: approach 1:
                if let Some(Event::Key(key)) = event {
                    match key.code {
                        KeyCode::Esc => self.stop_editing(),
                        _ => {
                            let input = &mut self.state.write().unwrap().input;
                            input.input(key);
                            info!("input.lines: {:?}", input.lines());
                        }
                    }
                }

                // TODO: approach 2:
                // match crossterm::event::read()?.into() {
                //     Input { key: Key::Esc, .. } => self.stop_editing(),
                //     input => {
                //         let mut input_val = self.state.read().unwrap().input.clone();
                //         info!("input: {:?}", input);
                //         input_val.input(input);
                //     }
                // }
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if action == Action::Tick {
            let mut input_state = self.state.write().unwrap();
            let cursor_color = if input_state.input_mode == InputMode::Editing {
                Color::Blue
            } else {
                Color::Red
            };
            input_state
                .input
                .set_cursor_style(Style::default().bg(cursor_color));
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<(), ErrReport> {
        let dictionary_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        };
        self.render_input(frame, dictionary_area);
        Ok(())
    }
}
