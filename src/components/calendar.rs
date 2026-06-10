use crate::components::Component;
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    prelude::{Color, Style},
    widgets::calendar::{CalendarEventStore, Monthly},
};
use time::OffsetDateTime;

#[derive(Debug)]
pub struct Calendar {}

impl Default for Calendar {
    fn default() -> Self {
        Self::new()
    }
}

impl Calendar {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for Calendar {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let calendar_area = Rect {
            x: area.x + 3,
            y: area.y + 6,
            width: 23,
            height: 5,
        };
        let date = OffsetDateTime::now_local()
            .unwrap_or(OffsetDateTime::now_utc())
            .date();
        let monthly = Monthly::new(date, CalendarEventStore::today(Style::new().red().bold()))
            .show_weekdays_header(Style::new().italic().fg(Color::Red));

        frame.render_widget(monthly, calendar_area);

        Ok(())
    }
}
