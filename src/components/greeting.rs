use super::Component;
use crate::app::LoadingStatus;
use chrono::Local;
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Default)]
pub struct LocationState {
    pub city: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
}

#[derive(Debug, Default, Clone)]
pub struct GreetingState {
    pub location: LocationState,
    pub loading_status: LoadingStatus,
}

#[derive(Clone)]
pub struct Greeting {
    pub state: Arc<RwLock<GreetingState>>,
}

impl Default for Greeting {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(GreetingState::default())),
        }
    }
}

impl Greeting {
    pub fn new() -> Self {
        let greeting = Self::default();
        greeting.run();
        greeting
    }

    fn run(&self) {
        let this = self.clone(); // clone the widget to pass to the background task
        tokio::spawn(async move {
            this.fetch_location_data().await;
        });
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    async fn fetch_location_data(&self) {
        self.set_loading_state(LoadingStatus::Loading);

        match self.get_public_ip().await {
            Ok(ip) => {
                tracing::info!("Public IP detected: {}", ip);

                let service = ipgeolocate::Service::IpApi;

                match ipgeolocate::Locator::get(&ip, service).await {
                    Ok(ip_info) => {
                        tracing::info!(
                            "IP Location found: {} - {} ({})",
                            ip_info.ip,
                            ip_info.city,
                            ip_info.country
                        );

                        let location_data = LocationState {
                            city: ip_info.city,
                            country: ip_info.country,
                            latitude: ip_info.latitude.parse().unwrap_or(0.0),
                            longitude: ip_info.longitude.parse().unwrap_or(0.0),
                            timezone: ip_info.timezone,
                        };
                        let mut state = self.state.write().unwrap();

                        state.location = location_data;
                        state.loading_status = LoadingStatus::Loaded;
                    }
                    Err(error) => {
                        tracing::error!("Error fetching IP location: {}", error);
                        self.set_loading_state(LoadingStatus::Error(error.to_string()));
                    }
                }
            }
            Err(error) => {
                tracing::error!("Error fetching public IP: {}", error);
                self.set_loading_state(LoadingStatus::Error(format!(
                    "Failed to get public IP: {error:?}",
                )));
            }
        }
    }

    async fn get_public_ip(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Try multiple IP API services in case one fails
        let ip_apis = [
            "https://api.ipify.org",
            "https://ifconfig.me/ip",
            "https://icanhazip.com",
        ];

        for api_url in ip_apis {
            match reqwest::get(api_url).await {
                Ok(response) => {
                    if let Ok(ip) = response.text().await {
                        if !ip.trim().is_empty() {
                            return Ok(ip.trim().to_string());
                        }
                    }
                }
                Err(_) => continue, // Try the next API if this one fails
            }
        }

        Err("Failed to get public IP".into())
    }

    fn get_location_display(&self) -> String {
        let state = self.state.read().unwrap();
        let location_loading = "🌐 Location is loading...".to_string();

        match state.loading_status {
            LoadingStatus::NotStarted => location_loading,
            LoadingStatus::Loading => location_loading,
            LoadingStatus::Loaded => {
                format!("🌐 {}, {}", state.location.city, state.location.country,)
            }
            LoadingStatus::Error(ref error) => format!("Location error: {error:?}"),
        }
    }
}

impl Component for Greeting {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let greeting_message = String::from("👋 ") + &whoami::username().to_ascii_uppercase();
        let now = Local::now();
        let datetime_str = now.format("%a, %b %d, %Y %H:%M:%S").to_string();
        let location_str = self.get_location_display();

        // Define area for the bordered content
        let border_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.saturating_sub(1),
        };
        // Create border widget with date as title
        let border_widget = Block::default()
            .title(datetime_str)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::LightCyan));
        // Define areas for content inside the border
        let greeting_area = Rect {
            x: area.x + 2, // Adjusted for border
            y: area.y + 2, // Adjusted for border and title
            width: area.width.min(30),
            height: 2,
        };
        let location_area = Rect {
            x: area.x + 2,             // Adjusted for border
            y: area.y + 3,             // Position below the greeting
            width: area.width.min(30), // Account for the border on both sides
            height: 1,
        };

        // Create content widgets
        let greeting_widget = Paragraph::new(greeting_message).style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        );
        let location_widget =
            Paragraph::new(location_str).style(Style::default().fg(Color::DarkGray));

        frame.render_widget(border_widget, border_area);
        frame.render_widget(greeting_widget, greeting_area);
        frame.render_widget(location_widget, location_area);

        Ok(())
    }
}
