use crate::replay::{ReplayStatus, ServiceState};

#[test]
fn replay_status_truthfully_reflects_save_readiness() {
    let mut status = ReplayStatus {
        state: ServiceState::Buffering,
        target: Some("game".to_string()),
        buffered_millis: 12500,
        buffered_bytes: 1024 * 1024 * 50,
        message: None,
    };
    assert!(status.can_save());

    // Retaining after exit retains savability while buffered frames remain
    status.state = ServiceState::RetainingAfterExit;
    assert!(status.can_save());

    // Empty buffer cannot be saved
    status.buffered_millis = 0;
    assert!(!status.can_save());

    // Waiting for target has no frames to save
    status.state = ServiceState::WaitingForTarget;
    assert!(!status.can_save());

    // Suspended or Failed cannot save
    status.state = ServiceState::Suspended;
    assert!(!status.can_save());
    status.state = ServiceState::Failed;
    assert!(!status.can_save());
}
