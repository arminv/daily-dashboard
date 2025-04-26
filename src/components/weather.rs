use crate::app::LoadingStatus;
use std::sync::{Arc, RwLock};

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
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(WeatherState::default())),
        }
    }
}

impl Weather {
    pub fn new() -> Self {
        let weather = Self::default();
        weather.run();
        weather
    }

    fn run(&self) {
        let this = self.clone(); // clone the widget to pass to the background task
        tokio::spawn(async move {
            this.fetch_weather_data().await;
        });
    }

    fn set_loading_state(&self, status: LoadingStatus) {
        let mut state = self.state.write().unwrap();
        state.loading_status = status;
    }

    // TODO: fetch weather data
    async fn fetch_weather_data(&self) {}
}
