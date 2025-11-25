use circles_types::Profile;

#[test]
fn decode_profile_batch() {
    let profiles: Vec<Profile> =
        serde_json::from_str(include_str!("fixtures/profile_batch.json")).expect("parse profiles");
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].name, "Alice");
    assert_eq!(profiles[1].name, "Bob");
}
