use super::*;
use crate::replay::{ReplayStatus, ServiceState};

#[test]
fn palette_output_prioritizes_pointer_over_compositor_and_fallback() {
    let outputs = vec![
        ("eDP-1".to_string(), 0, 0),
        ("DP-1".to_string(), 1920, 0),
        ("HDMI-A-1".to_string(), 3840, 0),
    ];

    // 1. Pointer output is chosen regardless of compositor active monitor
    assert_eq!(
        resolve_palette_output(Some(1), Some("HDMI-A-1"), &outputs),
        Some(1)
    );

    // 2. When pointer output is unknown, compositor active monitor is chosen
    assert_eq!(
        resolve_palette_output(None, Some("HDMI-A-1"), &outputs),
        Some(2)
    );

    // 3. When compositor active is unknown or cannot be mapped, (0, 0) is chosen
    assert_eq!(
        resolve_palette_output(None, Some("non-existent"), &outputs),
        Some(0)
    );
    assert_eq!(resolve_palette_output(None, None, &outputs), Some(0));

    // 4. Deterministic fallback when no output is at (0, 0)
    let shifted_outputs = vec![
        ("Z-Monitor".to_string(), 500, 500),
        ("A-Monitor".to_string(), 500, 500),
        ("B-Monitor".to_string(), 1000, 100),
    ];
    // Lowest x, y, then name -> A-Monitor at index 1
    assert_eq!(
        resolve_palette_output(None, None, &shifted_outputs),
        Some(1)
    );
}

#[test]
fn hud_output_prioritizes_selection_and_pointer_geometry_over_fallback() {
    let outputs = vec![
        (0, 0, 1920, 1080),
        (1920, 0, 2560, 1440),
        (-1920, 0, 1920, 1080),
    ];

    // Point on second monitor
    assert_eq!(
        resolve_hud_output(Some((2000, 500)), &outputs, Some(0)),
        Some(1)
    );

    // Point on third monitor (negative x coordinate)
    assert_eq!(
        resolve_hud_output(Some((-500, 300)), &outputs, Some(0)),
        Some(2)
    );

    // Point on first monitor
    assert_eq!(
        resolve_hud_output(Some((100, 100)), &outputs, Some(1)),
        Some(0)
    );

    // Point outside all monitors falls back to active palette output
    assert_eq!(
        resolve_hud_output(Some((10000, 10000)), &outputs, Some(1)),
        Some(1)
    );
    assert_eq!(resolve_hud_output(None, &outputs, Some(2)), Some(2));
}

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
