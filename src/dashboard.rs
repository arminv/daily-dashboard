use crate::components::{
    Component, calendar::Calendar, greeting::Greeting, inspiration::Inspiration, news::News,
    weather::Weather,
};
use color_eyre::{Result, eyre::Ok};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
};

pub struct Dashboard {
    components: Vec<Box<dyn Component>>,
}

impl Dashboard {
    pub fn new() -> Self {
        let calendar = Box::new(Calendar::new());
        let greeting = Box::new(Greeting::new());
        let weather = Box::new(Weather::new());
        let inspiration = Box::new(Inspiration::new());
        let news = Box::new(News::new());
        let components: Vec<Box<dyn Component>> =
            vec![calendar, greeting, weather, inspiration, news];
        Self { components }
    }
}

impl Component for Dashboard {
    // Since Dashboard is the only officially registered/orchestrator component, we need to pass along events, updates, etc. to child components
    fn handle_events(
        &mut self,
        event: Option<crate::tui::Event>,
    ) -> Result<Option<crate::action::Action>> {
        for component in &mut self.components {
            let _ = component.handle_events(event.clone());
        }
        Ok(None)
    }

    fn update(&mut self, action: crate::action::Action) -> Result<Option<crate::action::Action>> {
        for component in &mut self.components {
            let _ = component.update(action.clone());
        }
        let _ = action;
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(10), Constraint::Percentage(85)])
            .flex(Flex::SpaceBetween)
            .spacing(1)
            .split(area);
        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(30),
                Constraint::Min(30),
                Constraint::Min(30),
            ])
            .flex(Flex::SpaceBetween)
            .spacing(1)
            .split(outer_layout[0]);

        for (idx, component) in self.components.iter_mut().enumerate() {
            let target_layout = match idx {
                0 => inner_layout[0], // Calendar
                1 => inner_layout[0], // Greeting
                2 => inner_layout[1], // Weather
                3 => inner_layout[2], // Inspiration
                4 => outer_layout[1], // News
                _ => Rect::default(), // N/A
            };
            component.draw(frame, target_layout)?
        }
        Ok(())
    }
}
