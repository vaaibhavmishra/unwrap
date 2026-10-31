use super::{AppIcon, AppIconSettings};

#[test]
fn legacy_dock_preferences_resolve_to_unwrap_artwork() {
    for value in [
        "Aurora",
        "Classic1",
        "Classic2",
        "Classic3",
        "Comets",
        "Cow",
        "Glitch",
        "Holographic",
        "Neon",
        "Starburst",
        "Sticker",
        "WarpOne",
    ] {
        let icon: AppIcon = serde_json::from_value(serde_json::json!(value)).unwrap();
        assert_eq!(AppIconSettings::get_base_icon_file_name(icon), "warp_2");
    }
    for (value, expected) in [
        ("Glow", "dark"),
        ("Original", "dark"),
        ("GlassSky", "light"),
    ] {
        let icon: AppIcon = serde_json::from_value(serde_json::json!(value)).unwrap();
        assert_eq!(AppIconSettings::get_base_icon_file_name(icon), expected);
    }
}

#[test]
fn current_dock_preferences_round_trip() {
    for icon in [
        AppIcon::Default,
        AppIcon::Dark,
        AppIcon::Light,
        AppIcon::Mono,
    ] {
        let value = serde_json::to_value(icon).unwrap();
        let restored: AppIcon = serde_json::from_value(value).unwrap();
        assert_eq!(restored, icon);
    }
}
