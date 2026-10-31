use super::*;

#[test]
fn test_bundled_local_icon_exists() {
    assert!(Assets.get("bundled/png/local.png").is_ok());
}
