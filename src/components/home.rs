use color_eyre::Result;
use ratatui::{prelude::Modifier, prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // println!("tick");
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // println!("render");
                // add any logic here that should run on every render
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Text content
        let text = "WELCOME TO YOUR DAILY DASHBOARD 🌄";

        // Calculate a centered area for the text
        let center_rect = Rect {
            // Center horizontally
            x: area.x + (area.width.saturating_sub(text.len() as u16)) / 2,
            // Center vertically
            y: area.y + area.height / 2,
            // Exact width needed for the text
            width: text.len() as u16,
            height: 1,
        };
        let vertical = Layout::vertical([Constraint::Percentage(100)]);
        let [content] = vertical.areas(area);

        let block = Block::bordered();
        let paragraph = Paragraph::new(text)
            .add_modifier(Modifier::BOLD)
            .style(Style::default().fg(Color::LightYellow));

        frame.render_widget(block, content);
        frame.render_widget(paragraph, center_rect);
        Ok(())
    }
}
