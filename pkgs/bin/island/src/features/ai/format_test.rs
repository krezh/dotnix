use super::*;

#[test]
fn percent_reads_full_only_once_spent() {
    assert_eq!(percent(99.6), "99%");
    assert_eq!(percent(100.0), "100%");
    assert_eq!(percent(0.96), "0.9%");
    assert_eq!(percent(0.0), "0%");
}

#[test]
fn tooltip_counts_saved_resets_and_says_how_to_use_one() {
    use super::super::state::{Credits, Grant};
    use chrono::TimeZone;
    let mut provider = Provider {
        source: Source::Claude,
        id: "claude".to_owned(),
        provider_id: "anthropic".to_owned(),
        name: "Anthropic".to_owned(),
        account: String::new(),
        plan: String::new(),
        limits: Vec::new(),
        credits: None,
        fetched_at: 0,
        freshness: Freshness::Fresh,
    };
    assert_eq!(saved_resets(&provider), None);
    let expires = chrono::Local
        .with_ymd_and_hms(2026, 12, 10, 23, 0, 0)
        .single()
        .unwrap()
        .timestamp_millis();
    provider.credits = Some(Credits {
        count: 2,
        expires_at: Some(expires),
        next: None,
    });
    assert_eq!(
        saved_resets(&provider).as_deref(),
        Some("2 saved resets · expires Thu 10 Dec")
    );
    provider.credits = Some(Credits {
        count: 1,
        expires_at: None,
        next: Some(Grant {
            program: "cedar_ember".to_owned(),
            id: "full-reset-2611".to_owned(),
            label: String::new(),
            expires_at: None,
            clears: Vec::new(),
            used: Vec::new(),
        }),
    });
    assert_eq!(
        saved_resets(&provider).as_deref(),
        Some("1 saved reset · click the chip in the panel to use it")
    );
}
