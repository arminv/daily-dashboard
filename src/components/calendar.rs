use crate::action::Action;
use crate::components::Component;
use color_eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::{Color, Style, Stylize};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use time::OffsetDateTime;

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
        let calendar_area = Rect {
            x: area.x + 2,
            y: area.y + 14,
            width: area.width - 40,
            height: 8,
        };
        let date = OffsetDateTime::now_utc().date();
        let monthly = Monthly::new(date, CalendarEventStore::today(Style::new().red().bold()))
            // .block(Block::new().padding(Padding::new(0, 0, 2, 0)))
            .show_month_header(Style::new().bold().fg(Color::Red))
            .show_weekdays_header(Style::new().italic().fg(Color::Red));

        frame.render_widget(monthly, calendar_area);

        Ok(())
    }
}
