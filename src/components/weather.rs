use super::Component;
use crate::action::Action;
use crate::app::LoadingStatus;
use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use serde_json;
use std::sync::{Arc, RwLock};
use tracing::{error, info};

#[derive(Clone, Debug, Default)]
pub struct WeatherState {
    pub city: String,
    pub temperature: f32,
    pub description: String,
    pub icon: String,
    pub wind: String,
    pub loading_status: LoadingStatus,
}

#[derive(Clone)]
pub struct Weather {
    state: Arc<RwLock<WeatherState>>,
    greeting_state: Arc<RwLock<super::greeting::GreetingState>>,
}

impl Weather {
    pub fn new(greeting_state: Arc<RwLock<super::greeting::GreetingState>>) -> Self {
        Self {
            state: Arc::new(RwLock::new(WeatherState::default())),
            greeting_state,
        }
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    // Helper method to convert weather code to description
    fn get_weather_description(&self, code: u32) -> String {
        match code {
            0 => "Clear sky".to_string(),
            1 => "Mainly clear".to_string(),
            2 => "Partly cloudy".to_string(),
            3 => "Overcast".to_string(),
            45..=48 => "Fog".to_string(),
            51..=55 => "Drizzle".to_string(),
            61..=65 => "Rain".to_string(),
            71..=75 => "Snow".to_string(),
            80..=82 => "Rain showers".to_string(),
            95..=99 => "Thunderstorm".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    // Helper method to get weather icon
    fn get_weather_icon(&self, code: u32) -> String {
        match code {
            0 => "☀️".to_string(),
            1 | 2 => "🌤️".to_string(),
            3 => "☁️".to_string(),
            45..=48 => "🌫️".to_string(),
            51..=55 => "🌧️".to_string(),
            61..=65 => "🌧️".to_string(),
            71..=75 => "❄️".to_string(),
            80..=82 => "🌦️".to_string(),
            95..=99 => "⛈️".to_string(),
            _ => "🌡️".to_string(),
        }
    }

    async fn fetch_weather_data(&self) {
        self.set_loading_state(LoadingStatus::Loading);

        // Check if location data is ready
        let location_data = {
            let greeting_state = self.greeting_state.read().unwrap();
            if !matches!(greeting_state.loading_status, LoadingStatus::Loaded) {
                info!(
                    "Weather: Location not loaded yet (status: {:?}), will retry later",
                    greeting_state.loading_status
                );
                self.set_loading_state(LoadingStatus::NotStarted);
                return;
            }

            // Make a copy of the location data to release the lock
            (
                greeting_state.location.city.clone(),
                greeting_state.location.latitude,
                greeting_state.location.longitude,
            )
        };

        let (city, lat, lon) = location_data;
        let api_url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={lat:?}&longitude={lon:?}&current=temperature_2m,weather_code,wind_speed_10m",
        );
        let response = match reqwest::get(&api_url).await {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("API request failed: {e:?}");
                error!("Weather: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        if !response.status().is_success() {
            let error_msg = format!("API returned error status: {}", response.status());
            error!("Weather: {}", error_msg);
            self.set_loading_state(LoadingStatus::Error(error_msg));
            return;
        }

        // Get the response text
        let body_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                let error_msg = format!("Failed to read response body: {e:?}",);
                error!("Weather: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        // Parse the JSON
        let json: serde_json::Value = match serde_json::from_str(&body_text) {
            Ok(json) => json,
            Err(e) => {
                let error_msg = format!("Failed to parse JSON: {e:?}");
                error!("Weather: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        // Extract the weather data
        let current = match json.get("current") {
            Some(current) => current,
            None => {
                let error_msg = "No 'current' field in response".to_string();
                error!("Weather: {}", error_msg);
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let mut weather_state = self.state.write().unwrap();
        weather_state.city = city;

        // Get temperature
        if let Some(temp) = current.get("temperature_2m") {
            if let Some(value) = temp.as_f64() {
                weather_state.temperature = value as f32;
                info!("Weather: Temperature: {}", value);
            }
        }

        // Get wind speed
        if let Some(wind) = current.get("wind_speed_10m") {
            if let Some(value) = wind.as_f64() {
                weather_state.wind = format!("{value:.1} km/h");
                info!("Weather: Wind: {value:?}");
            }
        }

        // Get weather code and description
        if let Some(code) = current.get("weather_code") {
            if let Some(value) = code.as_u64() {
                let code_value = value as u32;
                weather_state.description = self.get_weather_description(code_value);
                weather_state.icon = self.get_weather_icon(code_value);
                info!("Weather: Code {}: {}", value, weather_state.description);
            }
        }

        // Mark as loaded
        weather_state.loading_status = LoadingStatus::Loaded;
        info!("Weather: Successfully loaded weather data");
    }

    fn get_weather_display(&self) -> String {
        let state = self.state.read().unwrap();
        let greeting_state = self.greeting_state.read().unwrap();

        match state.loading_status {
            LoadingStatus::NotStarted => "...".to_string(),
            LoadingStatus::Loading => "Weather: Loading...".to_string(),
            LoadingStatus::Loaded => {
                format!(
                    "{}{}{} {:.1}°C {} ({})",
                    state.icon,
                    state.icon,
                    state.icon,
                    state.temperature,
                    state.description,
                    state.wind
                )
            }
            LoadingStatus::Error(ref error) => {
                if !matches!(greeting_state.loading_status, LoadingStatus::Loaded) {
                    "Weather: Waiting for location data...".to_string()
                } else {
                    format!("Weather error: {error:?}")
                }
            }
        }
    }
}

impl Component for Weather {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if action == Action::Tick {
            let should_fetch = {
                let weather_state = self.state.read().unwrap();
                let greeting_state = self.greeting_state.read().unwrap();

                // Only fetch if:
                // 1. Location is loaded
                // 2. Weather is not loaded, or had an error, or is still loading for too long
                matches!(greeting_state.loading_status, LoadingStatus::Loaded)
                    && matches!(
                        weather_state.loading_status,
                        LoadingStatus::NotStarted | LoadingStatus::Error(_)
                    )
            };

            if should_fetch {
                let this = self.clone();
                tokio::spawn(async move {
                    info!("Weather: Trying fetch from tick event - location data is ready");
                    this.fetch_weather_data().await;
                });
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let weather_str = self.get_weather_display();
        let weather_area = Rect {
            x: area.x + 1,
            y: area.y + 3, // Position below location
            width: area.width,
            height: 1,
        };
        let weather_widget = Paragraph::new(weather_str).style(Style::default().fg(Color::Green));

        frame.render_widget(weather_widget, weather_area);
        Ok(())
    }
}
