use orbital_movement_gdextension::greet;

#[test]
fn greet_round_trip() {
    let message = greet("integration test").expect("greet should succeed");
    assert!(message.contains("integration test"));
}
