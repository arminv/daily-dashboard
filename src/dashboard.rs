use crate::components::{
    Component, calendar::Calendar, dictionary::Dictionary, greeting::Greeting,
    inspiration::Inspiration, news::News, weather::Weather,
};
use color_eyre::{Result, eyre::Ok};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
};

pub struct Dashboard {
    calendar: Calendar,
    greeting: Greeting,
    weather: Weather,
    inspiration: Inspiration,
    dictionary: Dictionary,
    news: News,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            calendar: Calendar::new(),
            greeting: Greeting::new(),
            weather: Weather::new(),
            inspiration: Inspiration::new(),
            dictionary: Dictionary::new(),
            news: News::new(),
        }
    }

    fn components(&mut self) -> [&mut dyn Component; 6] {
        [
            &mut self.calendar,
            &mut self.greeting,
            &mut self.weather,
            &mut self.inspiration,
            &mut self.dictionary,
            &mut self.news,
        ]
    }
}

impl Component for Dashboard {
    // Since Dashboard is the only officially registered/orchestrator component, we need to pass along events, updates, etc. to child components
    fn handle_events(
        &mut self,
        event: Option<crate::tui::Event>,
    ) -> Result<Option<crate::action::Action>> {
        // A component that is actively handling input (e.g. the Dictionary while
        // editing) returns `Some(Action::Render)` to signal that it consumed the
        // event. Stop propagating so sibling widgets (e.g. News, whose `Enter`
        // opens an article) don't also react to the same keypress.
        for component in self.components() {
            if component.handle_events(event.clone())?.is_some() {
                break;
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: crate::action::Action) -> Result<Option<crate::action::Action>> {
        for component in self.components() {
            let _ = component.update(action.clone());
        }
        let _ = action;
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // Top row holds the status widgets and the dictionary. The dictionary
        // starts at the very top of the right column and needs plenty of
        // vertical room to show the definition below its input, so the top row
        // gets a healthy share of the height. The left column stacks the
        // calendar (with the overlapping greeting) on top and the inspiration
        // quote below it.
        let page_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)])
            .flex(Flex::SpaceBetween)
            .spacing(1)
            .split(area);
        let top_row_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(30),
                Constraint::Min(30),
                Constraint::Min(30),
            ])
            .flex(Flex::SpaceBetween)
            .spacing(1)
            .split(page_layout[0]);
        // Left column: calendar (+ greeting) on top, inspiration below it.
        let left_col_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(11), Constraint::Length(4)])
            .spacing(1)
            .split(top_row_layout[0]);

        self.calendar.draw(frame, left_col_layout[0])?;
        self.greeting.draw(frame, left_col_layout[0])?;
        self.inspiration.draw(frame, left_col_layout[1])?;
        self.weather.draw(frame, top_row_layout[1])?;
        // Dictionary occupies the full right column, starting at the very top.
        self.dictionary.draw(frame, top_row_layout[2])?;
        self.news.draw(frame, page_layout[1])?;
        Ok(())
    }
}
