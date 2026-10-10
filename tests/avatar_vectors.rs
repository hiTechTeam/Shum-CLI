use shum_cli::avatar;

#[test]
fn compact_geometry_retains_eyes_mouth_and_shoulders() {
    let person = avatar::render_cells(0);
    let animal = avatar::render_cells(1);
    for (pixels, eye) in [(&person, [39, 35, 37, 255]), (&animal, [40, 37, 41, 255])] {
        assert_eq!(pixels.len(), 144);
        assert_eq!(pixels[5 * 12 + 5], eye);
        assert_eq!(pixels[5 * 12 + 7], eye);
        assert_ne!(pixels[5 * 12 + 6], eye, "eyes must stay separate");
        assert_ne!(pixels[11 * 12 + 2][3], 0, "left shoulder must remain");
        assert_ne!(pixels[11 * 12 + 9][3], 0, "right shoulder must remain");
        assert_eq!(pixels[0][3], 0, "transparent canvas must remain");
    }
    assert_eq!(person[7 * 12 + 5], [151, 75, 74, 255], "mouth must remain");
}
