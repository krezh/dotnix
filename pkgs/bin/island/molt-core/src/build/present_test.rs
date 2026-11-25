use super::*;

#[test]
fn error_summary_drops_molt_command_line() {
    assert_eq!(
        error_summary("nix build failed:\nerror: builder for '/nix/store/a-x.drv' failed"),
        "error: builder for '/nix/store/a-x.drv' failed"
    );
    assert_eq!(
        error_summary("nix eval returned an invalid derivation path: x"),
        "nix eval returned an invalid derivation path: x"
    );
}

#[test]
fn log_tail_keeps_the_end() {
    assert_eq!(log_tail("a\nb\n", 5), ("a\nb", 0));
    assert_eq!(log_tail("a\nb\nc\nd\n", 2), ("c\nd", 2));
    assert_eq!(log_tail("a\nb\nc", 1), ("c", 2));
}

#[test]
fn titles_follow_the_phase() {
    assert_eq!(
        phase_title(BuildPhase::Build, Target::NixOs),
        "Building NixOS"
    );
    assert_eq!(failed_title(BuildPhase::Evaluate), "Evaluation failed");
}
