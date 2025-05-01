use crate::action::Action;
use crate::components::Component;
use color_eyre::Result;
use ratatui::widgets::Block;
use ratatui::{
    Frame,
    layout::Rect,
    prelude::{Color, Style, Stylize},
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
    fn update(&mut self, _action: Action) -> Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let bordered_area = Rect {
            x: area.x + 2,
            y: area.y + 14,
            width: area.width.saturating_sub(4),
            height: 8,
        };
        let today = OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .date()
            .to_string()
            .bold()
            .into_centered_line();
        let bordered_block = Block::bordered().title(today);

        frame.render_widget(bordered_block, bordered_area);

        let calendar_area = Rect {
            x: area.x + 3,
            y: area.y + 15,
            width: 23,
            height: 5,
        };
        let date = OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .date();
        let monthly = Monthly::new(date, CalendarEventStore::today(Style::new().red().bold()))
            // .block(Block::new().padding(Padding::new(0, 0, 2, 0)))
            // .show_month_header(Style::new().bold().fg(Color::Red))
            .show_weekdays_header(Style::new().italic().fg(Color::Red));

        frame.render_widget(monthly, calendar_area);

        Ok(())
    }
}
