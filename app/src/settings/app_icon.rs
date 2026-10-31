use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use warp_core::settings::macros::define_settings_group;
use warp_core::settings::{SupportedPlatforms, SyncToCloud};

/// The app icon to use (mac-only).
///
/// IMPORTANT NOTE: If you add a new icon, you will need to update the logic in WarpDockTilePlugin.m
/// to read the new icon and also add the icon to app/DockTilePlugin/Resources.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Sequence,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "The app icon displayed in the dock.",
    rename_all = "snake_case"
)]
pub enum AppIcon {
    #[default]
    #[serde(
        alias = "Aurora",
        alias = "Classic1",
        alias = "Classic2",
        alias = "Classic3",
        alias = "Comets",
        alias = "Cow",
        alias = "Glitch",
        alias = "Holographic",
        alias = "Neon",
        alias = "Starburst",
        alias = "Sticker",
        alias = "WarpOne"
    )]
    Default,
    #[serde(alias = "Glow", alias = "Original")]
    Dark,
    #[serde(alias = "GlassSky")]
    Light,
    Mono,
}

impl std::fmt::Display for AppIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Default => "Default",
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Mono => "Mono",
        };
        write!(f, "{value}")
    }
}

impl AppIconSettings {
    pub fn get_base_icon_file_name(icon: AppIcon) -> &'static str {
        match icon {
            AppIcon::Default => "warp_2",
            AppIcon::Dark => "dark",
            AppIcon::Light => "light",
            AppIcon::Mono => "mono",
        }
    }
}

define_settings_group!(AppIconSettings, settings: [
    app_icon: AppIconState {
        type: AppIcon,
        default: AppIcon::Default,
        supported_platforms: SupportedPlatforms::MAC,
        sync_to_cloud: SyncToCloud::Never,
        surface: settings::SettingSurfaces::GUI,
        private: false,
        storage_key: "AppIcon",
        toml_path: "appearance.icon.app_icon",
        description: "The app icon displayed in the dock.",
    },
    show_dock_icon: ShowDockIconState {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::MAC,
        sync_to_cloud: SyncToCloud::Never,
        surface: settings::SettingSurfaces::GUI,
        private: false,
        storage_key: "ShowDockIcon",
        toml_path: "appearance.icon.show_dock_icon",
        description: "Whether Unwrap is shown in the macOS Dock and Cmd-Tab switcher.",
    },
]);

#[cfg(test)]
#[path = "app_icon_tests.rs"]
mod tests;
