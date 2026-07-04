use crate::{
    action::Action,
    components::{
        Component,
        calendar::{
            Calendar,
            MONTHLY_WIDTH,
        },
        dictionary::Dictionary,
        greeting::Greeting,
        inspiration::Inspiration,
        news::News,
        picture_frame::PictureFrame,
        weather::Weather,
    },
    config::Config,
    theme,
};
use color_eyre::{
    Result,
    eyre::Ok,
};
use ratatui::{
    Frame,
    layout::{
        Constraint,
        Direction,
        Flex,
        Layout,
        Rect,
        Size,
    },
};
use tokio::sync::mpsc::UnboundedSender;

pub struct Dashboard {
    calendar: Calendar,
    greeting: Greeting,
    weather: Weather,
    inspiration: Inspiration,
    dictionary: Dictionary,
    picture_frame: PictureFrame,
    news: News,
}

impl Dashboard {
    pub fn new(client: reqwest::Client) -> Self {
        // Build Greeting first so Weather can share its location state instead
        // of spawning a second, redundant geolocation lookup.
        let greeting = Greeting::new(client.clone());
        let weather = Weather::new(client.clone(), greeting.state.clone());
        // Detect the terminal's image graphics protocol + font size. Done here
        // (inside `App::new()`, before `tui.enter()` starts the crossterm event
        // loop) so `from_query_stdio`'s stdin query doesn't race the
        // `EventStream`. `DAILY_DASHBOARD_IMAGE_PROTOCOL` can override the
        // protocol; auto-detect falls back to universal halfblocks.
        let picker = PictureFrame::build_picker();
        let picture_frame = PictureFrame::new(client.clone(), picker);
        Self {
            calendar: Calendar::new(),
            greeting,
            weather,
            inspiration: Inspiration::new(client.clone()),
            dictionary: Dictionary::new(client.clone()),
            picture_frame,
            news: News::new(client),
        }
    }

    fn components(&mut self) -> [&mut dyn Component; 7] {
        [
            &mut self.calendar,
            &mut self.greeting,
            &mut self.weather,
            &mut self.inspiration,
            &mut self.dictionary,
            &mut self.picture_frame,
            &mut self.news,
        ]
    }
}

impl Component for Dashboard {
    // Since Dashboard is the only officially registered/orchestrator component, we need to pass
    // along events, updates, etc. to child components
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        for component in self.components() {
            component.register_action_handler(tx.clone())?;
        }
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        for component in self.components() {
            component.register_config_handler(config.clone())?;
        }
        Ok(())
    }

    fn init(&mut self, area: Size) -> Result<()> {
        for component in self.components() {
            component.init(area)?;
        }
        Ok(())
    }

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
        let left_col_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(11), Constraint::Length(6)])
            .spacing(1)
            .split(top_row_layout[0]);
        let calendar_panel = theme::frame_block("📅 Calendar");
        let calendar_inner = calendar_panel.inner(left_col_layout[0]);
        frame.render_widget(calendar_panel, left_col_layout[0]);

        const GREETING_MIN_WIDTH: u16 = 26;
        if calendar_inner.width >= GREETING_MIN_WIDTH + MONTHLY_WIDTH {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(GREETING_MIN_WIDTH),
                    Constraint::Length(MONTHLY_WIDTH),
                ])
                .split(calendar_inner);
            self.greeting.draw(frame, columns[0])?;
            self.calendar.draw(frame, columns[1])?;
        } else {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(calendar_inner);
            self.greeting.draw(frame, rows[0])?;
            self.calendar.draw(frame, rows[1])?;
        }
        self.inspiration.draw(frame, left_col_layout[1])?;
        self.weather.draw(frame, top_row_layout[1])?;
        self.picture_frame.draw(frame, top_row_layout[2])?;

        let bottom_row_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
            .flex(Flex::SpaceBetween)
            .spacing(1)
            .split(page_layout[1]);
        self.dictionary.draw(frame, bottom_row_layout[0])?;
        self.news.draw(frame, bottom_row_layout[1])?;
        Ok(())
    }
}
