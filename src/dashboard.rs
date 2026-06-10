use crate::components::Component;
use crate::components::calendar::Calendar;
use crate::components::greeting::Greeting;
use crate::components::news::News;
use crate::components::weather::Weather;
use color_eyre::Result;
use color_eyre::eyre::Ok;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{Frame, layout::Rect};

#[derive(Debug)]
pub struct Dashboard {
    calendar: Calendar,
    greeting: Greeting,
    weather: Weather,
    news: News,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Dashboard {
    pub fn new() -> Self {
        let calendar = Calendar::new();
        let greeting = Greeting::new();
        let weather = Weather::new();
        let news = News::new();

        Self {
            calendar,
            greeting,
            news,
            weather,
        }
    }
}

impl Component for Dashboard {
    fn handle_events(
        &mut self,
        event: Option<crate::tui::Event>,
    ) -> Result<Option<crate::action::Action>> {
        let _ = self.news.handle_events(event);
        Ok(None)
    }

    fn update(&mut self, action: crate::action::Action) -> Result<Option<crate::action::Action>> {
        let _ = self.news.update(action.clone());
        let _ = self.weather.update(action);
        let _ = action;
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let outer_layout_new = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(area);
        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(outer_layout_new[0]);

        let _calendar_widget = self.calendar.draw(frame, inner_layout[0]);
        let _greeting_widget = self.greeting.draw(frame, inner_layout[0]);
        let _weather_widget = self.weather.draw(frame, inner_layout[1]);
        let _weather_widget = self.news.draw(frame, outer_layout_new[1]);

        Ok(())
    }
}
