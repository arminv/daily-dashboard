use super::Component;
use super::greeting::GreetingState;
use crate::app::LoadingStatus;
use crate::{action::Action, components::greeting::Greeting};
use chrono::{Datelike, Local, NaiveDate};
use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use serde_json;
use std::sync::{Arc, RwLock};
use tracing::error;

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
    pub last_updated_at: Option<chrono::DateTime<Local>>,
}

#[derive(Clone, Debug)]
pub struct Weather {
    state: Arc<RwLock<WeatherState>>,
    greeting_state: Arc<RwLock<GreetingState>>,
}

const REFETCH_WEATHER_IN_MINS: i64 = 10;

impl Weather {
    pub fn new() -> Self {
        let greeting = Greeting::new();
        let greeting_state = greeting.state.clone();
        Self {
            state: Arc::new(RwLock::new(WeatherState::default())),
            greeting_state,
        }
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

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

        let location_data = {
            let greeting_state = self.greeting_state.read().unwrap();

            if !matches!(greeting_state.loading_status, LoadingStatus::Loaded) {
                self.set_loading_state(LoadingStatus::NotStarted);
                return;
            }

            // Make a copy of the location data to release the lock
            (
                greeting_state.location.city.clone(),
                greeting_state.location.latitude,
                greeting_state.location.longitude,
                greeting_state.location.timezone.clone(),
            )
        };

        let (city, lat, lon, timezone) = location_data;
        let api_url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={lat:?}&longitude={lon:?}&current=temperature_2m,weather_code,wind_speed_10m&daily=temperature_2m_max,temperature_2m_min&forecast_days=7&timezone={timezone}",
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

        let body_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                let error_msg = format!("Failed to read response body: {e:?}",);
                error!("Weather: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

        let json: serde_json::Value = match serde_json::from_str(&body_text) {
            Ok(json) => json,
            Err(e) => {
                let error_msg = format!("Failed to parse JSON: {e:?}");
                error!("Weather: {error_msg:?}");
                self.set_loading_state(LoadingStatus::Error(error_msg));
                return;
            }
        };

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

        if let Some(temp) = current.get("temperature_2m")
            && let Some(value) = temp.as_f64()
        {
            weather_state.temperature = value as f32;
        }

        if let Some(wind) = current.get("wind_speed_10m")
            && let Some(value) = wind.as_f64()
        {
            weather_state.wind = format!("{value:.1} km/h");
        }

        if let Some(code) = current.get("weather_code")
            && let Some(value) = code.as_u64()
        {
            let code_value = value as u32;
            weather_state.description = self.get_weather_description(code_value);
            weather_state.icon = self.get_weather_icon(code_value);
        }

        if let Some(daily) = json.get("daily") {
            weather_state.daily_weekdays.clear();
            weather_state.daily_high_temperatures.clear();
            weather_state.daily_low_temperatures.clear();

            let temp_max_array = daily.get("temperature_2m_max");
            let temp_min_array = daily.get("temperature_2m_min");
            let time_array = daily.get("time");

            if let Some(time_values) = time_array.and_then(|a| a.as_array()) {
                for time_value in time_values {
                    let date_str = time_value.as_str().unwrap_or("???");
                    weather_state.daily_weekdays.push(
                        NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                            .map(|date| date.weekday().to_string())
                            .unwrap_or("???".to_string()),
                    );
                }
            }

            if let Some(max_temps) = temp_max_array.and_then(|a| a.as_array()) {
                weather_state.daily_high_temperatures.extend(
                    max_temps
                        .iter()
                        .filter_map(|v| v.as_f64().map(|temp| temp as f32)),
                );
            }

            if let Some(min_temps) = temp_min_array.and_then(|a| a.as_array()) {
                weather_state.daily_low_temperatures.extend(
                    min_temps
                        .iter()
                        .filter_map(|v| v.as_f64().map(|temp| temp as f32)),
                );
            }
        }

        weather_state.loading_status = LoadingStatus::Loaded;
        weather_state.last_updated_at = Some(Local::now());
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
                    "{}{}{} {:.1}°C {} (wind: {})",
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

                let is_location_ready =
                    matches!(greeting_state.loading_status, LoadingStatus::Loaded);

                let is_initial_load = matches!(
                    weather_state.loading_status,
                    LoadingStatus::NotStarted | LoadingStatus::Error(_)
                );

                // Check if N minutes have passed since the last update
                let now = Local::now();
                let should_refresh = match weather_state.last_updated_at {
                    Some(last_updated) => {
                        let duration = now.signed_duration_since(last_updated);
                        // Refresh if more than N minutes have passed
                        duration.num_minutes() >= REFETCH_WEATHER_IN_MINS
                    }
                    None => true,
                };

                is_location_ready && (is_initial_load || should_refresh)
            };

            if should_fetch {
                let this = self.clone();
                tokio::spawn(async move {
                    this.fetch_weather_data().await;
                });
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let weather_str = self.get_weather_display();
        let weather_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height,
        };
        let weather_widget = Paragraph::new(weather_str).style(Style::default().fg(Color::Blue));
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
            x: main_area.x,
            y: main_area.y,
            width: main_area.width.saturating_sub(2),
            height: main_area.height,
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
        .value_style(Style::new().on_black().bold())
        .label_style(Style::new().fg(Color::Red))
        .bar_gap(1)
        .data(BarGroup::default().bars(&bars))
        .bar_width(6)
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
