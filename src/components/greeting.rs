use chrono::Local;
use color_eyre::Result;
use ratatui::{
    layout::Rect,
    prelude::*,
    style::{Modifier, Style},
    widgets::Paragraph,
};

use super::Component;

#[derive(Default)]
pub struct Greeting {}

impl Greeting {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for Greeting {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Prepare greeting text
        let greeting_message = String::from("Hello, ") + &whoami::realname() + "!";
        let now = Local::now();
        let datetime_str = now.format("%A, %B %d, %Y %H:%M:%S").to_string();

        let greeting_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width.min(30),
            height: 1,
        };

        let date_area = Rect {
            // Position on the right side of the screen
            x: area.x + area.width - datetime_str.len() as u16,
            y: area.y,
            width: datetime_str.len() as u16,
            height: 1,
        };

        let greeting_widget =
            Paragraph::new(greeting_message).style(Style::default().add_modifier(Modifier::BOLD));

        let date_widget =
            Paragraph::new(datetime_str).style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(greeting_widget, greeting_area);
        frame.render_widget(date_widget, date_area);

        Ok(())
    }
}
