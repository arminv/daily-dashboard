use super::*;
use crossterm::event::KeyCode;

#[test]
fn plain_keys_apply_when_not_capturing() {
    let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(should_dispatch_global_keymap(false, &q));
}

#[test]
fn plain_q_is_blocked_while_capturing_input() {
    let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(!should_dispatch_global_keymap(true, &q));
}

#[test]
fn ctrl_chords_still_apply_while_capturing_input() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    let ctrl_z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert!(should_dispatch_global_keymap(true, &ctrl_c));
    assert!(should_dispatch_global_keymap(true, &ctrl_d));
    assert!(should_dispatch_global_keymap(true, &ctrl_z));
}

#[test]
fn alt_chords_still_apply_while_capturing_input() {
    let alt_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT);
    assert!(should_dispatch_global_keymap(true, &alt_q));
}
