use super::*;

#[test]
fn parse_articles_preserves_category_order() {
    // JSON key order is deliberately reversed relative to NEWS_CATEGORIES; the
    // parser must emit Business (first in NEWS_CATEGORIES) before Entertainment.
    let json = serde_json::json!({
        "Entertainment": [{ "title": "E1", "link": "l", "source": "s" }],
        "Business": [{ "title": "B1", "link": "l", "source": "s" }],
    });
    let articles = parse_articles(&json);
    assert_eq!(articles.len(), 2);
    assert_eq!(articles[0].category, "Business");
    assert_eq!(articles[0].title, "B1");
    assert_eq!(articles[1].category, "Entertainment");
}

#[test]
fn parse_articles_skips_entries_missing_required_fields() {
    let json = serde_json::json!({
        "Business": [
            { "title": "Good", "link": "l", "source": "s" },
            { "title": "NoLink", "source": "s" },
            { "link": "l", "source": "s" },
            { "title": "NoSource", "link": "l" },
        ],
    });
    let articles = parse_articles(&json);
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].title, "Good");
}

#[test]
fn parse_articles_caps_each_category_at_limit() {
    let many: Vec<serde_json::Value> = (0..40)
        .map(|i| serde_json::json!({ "title": format!("T{i}"), "link": "l", "source": "s" }))
        .collect();
    let json = serde_json::json!({ "Business": many });
    let articles = parse_articles(&json);
    assert_eq!(articles.len(), MAX_NUMBER_OF_ARTICLES_FROM_EACH_CATEGORY);
    assert_eq!(articles[0].title, "T0");
    assert_eq!(
        articles.last().unwrap().title,
        format!("T{}", MAX_NUMBER_OF_ARTICLES_FROM_EACH_CATEGORY - 1)
    );
}

#[test]
fn parse_articles_ignores_empty_and_unknown_categories() {
    let json = serde_json::json!({
        "Business": [],
        "Weather": [{ "title": "X", "link": "l", "source": "s" }],
    });
    let articles = parse_articles(&json);
    assert!(articles.is_empty());
}
