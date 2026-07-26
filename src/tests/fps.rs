use super::*;

#[test]
fn is_enabled_value_accepts_truthy_forms() {
    for value in ["1", "true", "TRUE", "Yes", " on ", "ON"] {
        assert!(is_enabled_value(value), "{value:?} should enable FPS");
    }
}

#[test]
fn is_enabled_value_rejects_falsy_and_empty() {
    for value in ["", "0", "false", "FALSE", "no", "off", "maybe", "2"] {
        assert!(!is_enabled_value(value), "{value:?} should not enable FPS");
    }
}
