use player::PlayerOptions;

#[test]
fn preserves_valid_initial_volume() {
    let options = PlayerOptions::new(40);

    assert_eq!(options.initial_volume(), 40);
    assert_eq!(options.initial_volume_f32(), 0.4);
}

#[test]
fn clamps_initial_volume_to_one_hundred() {
    let options = PlayerOptions::new(150);

    assert_eq!(options.initial_volume(), 100);
    assert_eq!(options.initial_volume_f32(), 1.0);
}
