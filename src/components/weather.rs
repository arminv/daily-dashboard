use super::Component;
use crate::action::Action;
use crate::app::LoadingStatus;
use chrono::{Datelike, NaiveDate};
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
    pub daily_high_temperatures: Vec<f32>,
    pub daily_low_temperatures: Vec<f32>,
    pub daily_weekdays: Vec<String>,
}

#[derive(Clone, Debug)]
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
            "https://api.open-meteo.com/v1/forecast?latitude={lat:?}&longitude={lon:?}&current=temperature_2m,weather_code,wind_speed_10m&daily=temperature_2m_max,temperature_2m_min&forecast_days=7",
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

        if let Some(temp) = current.get("temperature_2m") {
            if let Some(value) = temp.as_f64() {
                weather_state.temperature = value as f32;
                info!("Weather: Temperature: {}", value);
            }
        }

        if let Some(wind) = current.get("wind_speed_10m") {
            if let Some(value) = wind.as_f64() {
                weather_state.wind = format!("{value:.1} km/h");
                info!("Weather: Wind: {value:?}");
            }
        }

        if let Some(code) = current.get("weather_code") {
            if let Some(value) = code.as_u64() {
                let code_value = value as u32;
                weather_state.description = self.get_weather_description(code_value);
                weather_state.icon = self.get_weather_icon(code_value);
                info!("Weather: Code {}: {}", value, weather_state.description);
            }
        }

        // Extract daily forecast data
        if let Some(daily) = json.get("daily") {
            let temp_max_array = daily.get("temperature_2m_max");
            let temp_min_array = daily.get("temperature_2m_min");
            let time_array = daily.get("time");

            // Process dates and weekdays
            if let (Some(_), Some(time_values)) =
                (time_array, time_array.and_then(|a| a.as_array()))
            {
                for time_value in time_values {
                    let date_str = time_value.as_str().unwrap_or("???");

                    if date_str.len() >= 10 {
                        // Get the weekday name
                        weather_state.daily_weekdays.push(
                            NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                                .map(|date| date.weekday().to_string())
                                .unwrap_or_else(|_| "???".to_string()),
                        );
                    } else {
                        weather_state.daily_weekdays.push("???".to_string());
                    }
                }
            }

            if let Some(max_temps) = temp_max_array.and_then(|a| a.as_array()) {
                weather_state
                    .daily_high_temperatures
                    .extend(max_temps.iter().filter_map(|v| {
                        v.as_f64().map(|temp| {
                            info!("Weather: Daily max temp: {}", temp);
                            temp as f32
                        })
                    }));
            }

            if let Some(min_temps) = temp_min_array.and_then(|a| a.as_array()) {
                weather_state
                    .daily_low_temperatures
                    .extend(min_temps.iter().filter_map(|v| {
                        v.as_f64().map(|temp| {
                            info!("Weather: Daily min temp: {}", temp);
                            temp as f32
                        })
                    }));
            }
        }

        weather_state.loading_status = LoadingStatus::Loaded;
        info!("Weather: Successfully loaded weather data");
    }

    fn get_weather_display(&self) -> String {
        let state = self.state.read().unwrap();
        let greeting_state = self.greeting_state.read().unwrap();
        let location_loading = "🌄 Weather is loading...".to_string();

        match state.loading_status {
            LoadingStatus::NotStarted => location_loading,
            LoadingStatus::Loading => location_loading,
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
            x: area.x + 2,
            y: area.y + 4, // Position below location
            width: area.width - 2,
            height: 1,
        };
        let weather_widget = Paragraph::new(weather_str).style(Style::default().fg(Color::Green));
        frame.render_widget(weather_widget, weather_area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Space for upper content
                Constraint::Length(0), // Chart title
                Constraint::Length(8), // Chart main area
            ])
            .split(area);
        let main_area = layout[2];
        let padded_chart_area = Rect {
            x: main_area.x + 1, // Add left padding
            y: main_area.y + 2,
            width: main_area.width.saturating_sub(2),
            ..main_area
        };

        let has_forecast_data = {
            let state = match self.state.read() {
                Ok(daily_high_temperatures) => daily_high_temperatures,
                Err(_) => {
                    eprintln!("Weather: Failed to read state");
                    return Ok(());
                }
            };
            !state.daily_high_temperatures.is_empty()
                && matches!(state.loading_status, LoadingStatus::Loaded)
        };

        if has_forecast_data {
            let (high_temps, low_temps, weekdays) = {
                let state = match self.state.read() {
                    Ok(state) => state,
                    Err(_) => {
                        eprintln!("Weather: Failed to read state");
                        return Ok(());
                    }
                };
                (
                    state.daily_high_temperatures.clone(),
                    state.daily_low_temperatures.clone(),
                    state.daily_weekdays.clone(),
                )
            };

            frame.render_widget(
                vertical_barchart(&high_temps, &low_temps, &weekdays),
                padded_chart_area,
            );
        }
        Ok(())
    }
}

fn vertical_barchart(
    high_temps: &[f32],
    low_temps: &[f32],
    weekdays: &[String],
) -> BarChart<'static> {
    let bars: Vec<Bar> = high_temps
        .iter()
        .enumerate()
        .map(|(index, high_temp)| {
            let low_temp = low_temps[index];
            vertical_bar(index, high_temp, &low_temp, weekdays)
        })
        .collect();

    BarChart::default()
        .block(Block::bordered().title("📈 7-Day Forecast (Low/High°)".bold().into_centered_line()))
        .value_style(Style::new().on_black().bold())
        .label_style(Style::new().fg(Color::Red))
        .bar_gap(1)
        .data(BarGroup::default().bars(&bars))
        .bar_width(5)
}

fn vertical_bar(
    index: usize,
    high_temp: &f32,
    low_temp: &f32,
    weekdays: &[String],
) -> Bar<'static> {
    // For display, round temperatures to integers
    let high_display = high_temp.round() as i32;
    let low_display = low_temp.round() as i32;
    let weekday = if index < weekdays.len() {
        &weekdays[index]
    } else {
        "?"
    };
    let label = Line::from(weekday.to_string()).alignment(Alignment::Center);
    let text_value = format!("{low_display}-{high_display}°");

    Bar::default()
        .value(*high_temp as u64)
        .label(label)
        .text_value(text_value)
        .value_style(Style::new().fg(Color::LightYellow))
        .style(temperature_style(*high_temp))
}

/// Create a color gradient based on temperature
/// - Cold temperatures (below 0°C): Blue
/// - Moderate temperatures (0-20°C): Green to Yellow
/// - Warm temperatures (20-30°C): Yellow to Orange
/// - Hot temperatures (above 30°C): Orange to Red
fn temperature_style(value: f32) -> Style {
    let (r, g, b) = if value < 0.0 {
        // Cold: Blue
        (50, 50, 255)
    } else if value < 10.0 {
        // Cool: Blue-Green
        let blue = (255.0 * (1.0 - value / 10.0)) as u8;
        let green = (200.0 * (value / 10.0)) as u8;
        (0, green, blue)
    } else if value < 20.0 {
        // Mild: Green-Yellow
        let green = 200;
        let red = (255.0 * ((value - 10.0) / 10.0)) as u8;
        (red, green, 0)
    } else if value < 30.0 {
        // Warm: Yellow-Orange
        let green = (200.0 * (1.0 - (value - 20.0) / 10.0)) as u8;
        (255, green, 0)
    } else {
        // Hot: Orange-Red
        let green = (100.0 * (1.0 - (value - 30.0).min(10.0) / 10.0)) as u8;
        (255, green, 0)
    };

    Style::new().fg(Color::Rgb(r, g, b))
}
