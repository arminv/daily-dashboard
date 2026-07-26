use super::*;
use chrono::{
    Duration,
    Local,
    TimeZone,
};

fn fixed_now() -> DateTime<Local> {
    Local.with_ymd_and_hms(2026, 7, 25, 12, 0, 0).unwrap()
}

#[test]
fn not_started_is_always_due() {
    let status = LoadingStatus::NotStarted;
    assert!(status.is_fetch_due(fixed_now(), None, Some(30), 1));
    assert!(status.is_fetch_due(fixed_now(), Some(fixed_now()), None, 1));
}

#[test]
fn loading_is_never_due() {
    let status = LoadingStatus::Loading;
    assert!(!status.is_fetch_due(fixed_now(), None, Some(30), 1));
}

#[test]
fn loaded_respects_refetch_interval() {
    let now = fixed_now();
    let recent = now - Duration::minutes(5);
    let stale = now - Duration::minutes(30);
    let status = LoadingStatus::Loaded;

    assert!(!status.is_fetch_due(now, Some(recent), Some(30), 1));
    assert!(status.is_fetch_due(now, Some(stale), Some(30), 1));
    assert!(status.is_fetch_due(now, None, Some(30), 1));
}

#[test]
fn loaded_with_no_refetch_interval_is_never_due() {
    let now = fixed_now();
    let status = LoadingStatus::Loaded;
    assert!(!status.is_fetch_due(now, None, None, 1));
    assert!(!status.is_fetch_due(now, Some(now - Duration::minutes(120)), None, 1));
}

#[test]
fn error_respects_retry_interval() {
    let now = fixed_now();
    let recent = now - Duration::seconds(30);
    let stale = now - Duration::minutes(1);
    let status = LoadingStatus::Error("boom".to_string());

    assert!(!status.is_fetch_due(now, Some(recent), Some(30), 1));
    assert!(status.is_fetch_due(now, Some(stale), Some(30), 1));
    assert!(status.is_fetch_due(now, None, Some(30), 1));
}

#[test]
fn begin_fetch_if_due_marks_loading_only_when_due() {
    let now = fixed_now();

    let mut status = LoadingStatus::NotStarted;
    assert!(status.begin_fetch_if_due(now, None, Some(30), 1));
    assert_eq!(status, LoadingStatus::Loading);

    let mut status = LoadingStatus::Loaded;
    assert!(!status.begin_fetch_if_due(now, Some(now), Some(30), 1));
    assert_eq!(status, LoadingStatus::Loaded);
}
