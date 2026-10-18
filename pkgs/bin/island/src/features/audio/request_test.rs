use super::stepped;

const STEP: f32 = 0.05;

fn sweep(start: f32, delta: f32, steps: usize) -> Vec<f32> {
    let mut level = start;
    (0..steps)
        .map(|_| {
            level = stepped(level, delta);
            level
        })
        .collect()
}

#[test]
fn reaches_the_ends_from_off_grid_levels() {
    assert_eq!(stepped(0.04, -STEP), 0.0);
    assert_eq!(stepped(0.97, STEP), 1.0);
    assert_eq!(stepped(0.01, -STEP), 0.0);
    assert_eq!(stepped(0.99, STEP), 1.0);
}

#[test]
fn snaps_to_the_grid() {
    assert_eq!(stepped(0.47, STEP), 0.5);
    assert_eq!(stepped(0.47, -STEP), 0.45);
    assert_eq!(stepped(0.5, STEP), 0.55);
    assert_eq!(stepped(0.5, -STEP), 0.45);
}

#[test]
fn clamps_at_the_ends() {
    assert_eq!(stepped(0.0, -STEP), 0.0);
    assert_eq!(stepped(1.0, STEP), 1.0);
    assert_eq!(stepped(1.3, STEP), 1.0);
}

#[test]
fn fast_sweeps_hit_every_step_and_stop_at_the_ends() {
    let up = sweep(0.0, STEP, 30);
    assert_eq!(up[0], 0.05);
    assert_eq!(up[19], 1.0);
    assert!(up.windows(2).all(|pair| pair[1] - pair[0] <= STEP + 1e-6));
    assert_eq!(*up.last().unwrap(), 1.0);
    let down = sweep(1.0, -STEP, 30);
    assert_eq!(down[19], 0.0);
    assert_eq!(*down.last().unwrap(), 0.0);
}

#[test]
fn multi_notch_steps_move_by_whole_steps() {
    assert_eq!(stepped(0.5, 3.0 * STEP), 0.65);
    assert_eq!(stepped(0.04, -3.0 * STEP), 0.0);
}
