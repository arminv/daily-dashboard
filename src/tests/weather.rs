use super::*;

#[test]
fn parse_daily_forecast_extracts_weekdays_and_temps() {
    let json = serde_json::json!({
        "daily": {
            "time": ["2026-07-01", "2026-07-02", "2026-07-03"],
            "temperature_2m_max": [30.0, 28.5, 27.0],
            "temperature_2m_min": [15.0, 14.0, 13.0],
        }
    });
    let (weekdays, highs, lows) = parse_daily_forecast(&json);
    assert_eq!(weekdays.len(), 3);
    assert_eq!(highs.len(), 3);
    assert_eq!(lows.len(), 3);
    // Weekday names render as 3-letter abbreviations; don't pin the exact day
    // so the test stays date-stable.
    assert!(weekdays.iter().all(|w| w.len() == 3));
    assert!((highs[0] - 30.0).abs() < 1e-6);
    assert!((highs[1] - 28.5).abs() < 1e-6);
    assert!((lows[2] - 13.0).abs() < 1e-6);
}

#[test]
fn parse_daily_forecast_missing_daily_returns_empty() {
    let json = serde_json::json!({ "current": { "temperature_2m": 20.0 } });
    let (weekdays, highs, lows) = parse_daily_forecast(&json);
    assert!(weekdays.is_empty());
    assert!(highs.is_empty());
    assert!(lows.is_empty());
}

#[test]
fn parse_daily_forecast_bad_date_becomes_placeholder() {
    let json = serde_json::json!({
        "daily": {
            "time": ["not-a-date"],
            "temperature_2m_max": [1.0],
            "temperature_2m_min": [0.0],
        }
    });
    let (weekdays, highs, lows) = parse_daily_forecast(&json);
    assert_eq!(weekdays, vec!["???".to_string()]);
    assert!((highs[0] - 1.0).abs() < 1e-6);
    assert!((lows[0] - 0.0).abs() < 1e-6);
}

#[test]
fn parse_daily_forecast_tolerates_missing_temp_arrays() {
    let json = serde_json::json!({ "daily": { "time": ["2026-07-01"] } });
    let (weekdays, highs, lows) = parse_daily_forecast(&json);
    assert_eq!(weekdays.len(), 1);
    assert!(highs.is_empty());
    assert!(lows.is_empty());
}
