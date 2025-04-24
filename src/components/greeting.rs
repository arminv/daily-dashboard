use chrono::Local;
use color_eyre::Result;
use ratatui::prelude::*;

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
        let greeting_message = String::from("Hello, ") + &whoami::realname() + "!";
        let now = Local::now();
        let datetime_str = now.format("%A, %B %d, %Y %H:%M:%S").to_string();
        let greeting = Line::from(greeting_message + " Today is: " + &*datetime_str)
            .centered()
            .bold();

        frame.render_widget(greeting, area);
        Ok(())
    }
}
