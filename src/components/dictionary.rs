use crate::{action::Action, components::Component, tui::Event};
use color_eyre::eyre::ErrReport;
use crossterm::event::{self, KeyCode};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph},
};
use std::sync::{Arc, RwLock};
use tracing::info;
use tui_input::{Input, backend::crossterm::EventHandler};

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
    /// Current value of the input box
    input: Input,
    /// Current input mode
    input_mode: InputMode,
}

#[derive(Clone, Debug)]
pub struct Dictionary {
    state: Arc<RwLock<DictionaryState>>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(DictionaryState {
                input_mode: InputMode::Normal,
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

    // TODO:
    fn render_input(&self, frame: &mut Frame, area: Rect) {
        // keep 2 for borders and 1 for cursor
        let width = area.width;
        let state_read = self.state.read().unwrap();
        let scroll = state_read.input.visual_scroll(width as usize);
        let style = match state_read.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Color::Yellow.into(),
        };
        let input = Paragraph::new(state_read.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, area);

        if state_read.input_mode == InputMode::Editing {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = state_read.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }
}

impl Component for Dictionary {
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<Option<Action>> {
        let input_mode = self.state.read().unwrap().input_mode;
        match input_mode {
            InputMode::Normal => match event {
                Some(Event::Key(key)) => match key.code {
                    KeyCode::Esc => self.start_editing(),
                    _ => (),
                },
                _ => (),
            },
            InputMode::Editing => match event {
                Some(Event::Key(key)) => match key.code {
                    KeyCode::Esc => self.stop_editing(),
                    _ => {
                        info!("key.code - C : {:?}", key.code);

                        let mut state_write = self.state.write().unwrap();
                        let event = event::read()?;
                        state_write.input.handle_event(&event);
                        info!("Value : {:?}", state_write.input.value());
                    }
                },
                _ => (),
            },
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
