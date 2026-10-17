use super::Edge;

fn edge() -> Edge {
    Edge {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 30.0,
        radius: 10.0,
    }
}

fn close((ax, ay): (f32, f32), (bx, by): (f32, f32)) -> bool {
    (ax - bx).abs() < 0.01 && (ay - by).abs() < 0.01
}

#[test]
fn starts_at_the_top_edge_after_the_corner() {
    assert!(close(edge().point(0.0), (20.0, 20.0)));
}

#[test]
fn goes_round_clockwise_and_back_to_the_start() {
    let edge = edge();
    let across = 80.0 / edge.length();
    assert!(close(edge.point(across), (100.0, 20.0)));
    assert!(close(edge.point(across / 2.0), (60.0, 20.0)));
    assert!(close(edge.point(1.0), edge.point(0.0)));
    assert!(close(edge.point(-0.25), edge.point(0.75)));
}

#[test]
fn stays_on_the_edge() {
    let edge = edge();
    for step in 0..400 {
        let (x, y) = edge.point(step as f32 / 400.0);
        assert!((10.0 - 0.01..=110.01).contains(&x) && (20.0 - 0.01..=50.01).contains(&y), "{x}, {y}");
    }
}

#[test]
fn moves_evenly() {
    let edge = edge();
    let steps = 1000;
    let expected = edge.length() / steps as f32;
    for step in 0..steps {
        let (ax, ay) = edge.point(step as f32 / steps as f32);
        let (bx, by) = edge.point((step + 1) as f32 / steps as f32);
        let moved = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
        assert!((moved - expected).abs() < expected * 0.05, "step {step}: {moved} vs {expected}");
    }
}
