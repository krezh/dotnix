use super::*;

#[test]
fn save_requests_have_no_response_deadline() {
    let command = ReplayCommand::Save { output: None };

    assert_eq!(response_timeout(&command), None);
}

#[test]
fn short_control_requests_keep_a_bounded_response_deadline() {
    assert_eq!(
        response_timeout(&ReplayCommand::Status),
        Some(Duration::from_secs(30))
    );
}
