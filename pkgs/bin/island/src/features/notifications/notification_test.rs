use std::collections::HashMap;

use super::claim_from;

#[test]
fn a_fixed_id_keeps_replacing() {
    let mut owners = HashMap::new();
    let unused = || -> u32 { panic!("no fresh id is needed") };
    assert_eq!(claim_from(&mut owners, 2593, "volume", unused), 2593);
    assert_eq!(claim_from(&mut owners, 2593, "volume", unused), 2593);
}

#[test]
fn another_apps_id_is_not_taken() {
    let mut owners = HashMap::from([(5, "mail".to_owned())]);
    assert_eq!(claim_from(&mut owners, 5, "chat", || 6), 6);
    assert_eq!(owners[&5], "mail");
    assert_eq!(owners[&6], "chat");
}

#[test]
fn fresh_ids_skip_what_is_held() {
    let mut owners = HashMap::from([(1, "mail".to_owned()), (2, "chat".to_owned())]);
    let mut next = 0;
    let fresh = || {
        next += 1;
        next
    };
    assert_eq!(claim_from(&mut owners, 0, "new", fresh), 3);
}
