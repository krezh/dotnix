use super::{Security, security};

#[test]
fn access_point_flags_read_as_what_joining_needs() {
    assert_eq!(security(0, 0, 0), Security::Open);
    assert_eq!(security(0x1, 0, 0x100), Security::Wpa2);
    assert_eq!(security(0x1, 0, 0x400), Security::Wpa3);
    assert_eq!(security(0x1, 0, 0x500), Security::Wpa3);
    assert_eq!(security(0x1, 0x200, 0), Security::Enterprise);
}
