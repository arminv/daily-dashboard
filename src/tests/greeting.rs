use super::*;

#[test]
fn parse_location_extracts_fields_on_success() {
    let json = serde_json::json!({
        "status": "success",
        "city": "Mountain View",
        "country": "United States",
        "lat": 37.4192,
        "lon": -122.0574,
        "timezone": "America/Los_Angeles",
    });
    let location = parse_location(&json).expect("success response should parse");
    assert_eq!(location.city, "Mountain View");
    assert_eq!(location.country, "United States");
    assert!((location.latitude - 37.4192).abs() < 1e-6);
    assert!((location.longitude - (-122.0574)).abs() < 1e-6);
    assert_eq!(location.timezone, "America/Los_Angeles");
}

#[test]
fn parse_location_fail_status_returns_api_message() {
    let json = serde_json::json!({
        "status": "fail",
        "message": "invalid query",
    });
    let err = parse_location(&json).expect_err("fail status should error");
    assert!(
        err.to_string().contains("invalid query"),
        "error should include API message: {err}"
    );
}

#[test]
fn parse_location_missing_fields_default_to_empty() {
    let json = serde_json::json!({ "status": "success" });
    let location = parse_location(&json).expect("success with missing fields should parse");
    assert_eq!(location.city, "");
    assert_eq!(location.country, "");
    assert_eq!(location.latitude, 0.0);
    assert_eq!(location.longitude, 0.0);
    assert_eq!(location.timezone, "");
}
