use super::service;

fn uuids(list: &[&str]) -> Vec<String> {
    list.iter().map(|uuid| format!("{uuid}-0000-1000-8000-00805f9b34fb")).collect()
}

#[test]
fn a_device_is_named_for_what_it_is_used_for() {
    assert_eq!(service(&uuids(&["0000110b", "0000111e"])), "Music");
    assert_eq!(service(&uuids(&["00001124"])), "Input");
    assert_eq!(service(&uuids(&["00001812"])), "Input");
    assert_eq!(service(&uuids(&["0000111e"])), "Calls");
    assert_eq!(service(&uuids(&["00001200"])), "");
}
