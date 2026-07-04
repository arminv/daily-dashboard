use crate::components::Component;
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    prelude::{
        Color,
        Style,
    },
    widgets::calendar::{
        CalendarEventStore,
        Monthly,
    },
};
use time::OffsetDateTime;

#[derive(Debug, Default)]
pub struct Calendar {}

impl Calendar {
    pub fn new() -> Self {
        Self {}
    }
}

/// Natural size of the `Monthly` grid (weekday header + week rows). The grid is
/// placed at the top-right of the area the Dashboard allocates, so it never
/// relies on hardcoded offsets and resizes cleanly. `MONTHLY_WIDTH` is exposed
/// so the Dashboard can reserve a column for it.
pub(crate) const MONTHLY_WIDTH: u16 = 23;
const MONTHLY_HEIGHT: u16 = 5;

impl Component for Calendar {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let date = OffsetDateTime::now_local()
            .unwrap_or(OffsetDateTime::now_utc())
            .date();
        let monthly = Monthly::new(date, CalendarEventStore::today(Style::new().red().bold()))
            .show_weekdays_header(Style::new().italic().fg(Color::Red));
        let grid = Rect {
            x: area.x + area.width.saturating_sub(MONTHLY_WIDTH),
            y: area.y,
            width: area.width.min(MONTHLY_WIDTH),
            height: area.height.min(MONTHLY_HEIGHT),
        };
        frame.render_widget(monthly, grid);
        Ok(())
    }
}
