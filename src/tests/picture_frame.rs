use super::*;
use crossterm::event::KeyModifiers;

#[test]
fn image_url_builds_random_picsum_url() {
    assert_eq!(image_url(), "https://picsum.photos/1200/800");
}

#[test]
fn is_new_image_key_matches_shift_n() {
    // Shift+N produces the uppercase 'N' character.
    let key = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT);
    assert!(is_new_image_key(&key));
}

#[test]
fn is_new_image_key_matches_uppercase_n_without_modifier_flag() {
    // Some terminals report the uppercase char without an explicit SHIFT flag.
    let key = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::NONE);
    assert!(is_new_image_key(&key));
}

#[test]
fn is_new_image_key_rejects_plain_n() {
    let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
    assert!(!is_new_image_key(&key));
}

#[test]
fn is_new_image_key_rejects_ctrl_n() {
    // Ctrl / Ctrl+Shift with N is reported as lowercase 'n', so it must not fire.
    let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
    assert!(!is_new_image_key(&key));
}

#[test]
fn is_new_image_key_rejects_non_n_key() {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
    assert!(!is_new_image_key(&key));
}
