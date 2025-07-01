//! Various non-themable constants for the Feathers look and feel.

/// Font asset paths
pub mod fonts {
    /// Default regular font path
    pub const REGULAR: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-Regular.ttf";
    /// Regular italic font path
    pub const ITALIC: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-Italic.ttf";
    /// Bold font path
    pub const BOLD: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-Bold.ttf";
    /// Bold italic font path
    pub const BOLD_ITALIC: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-BoldItalic.ttf";
    /// Monospace font path
    pub const MONO: &str = "embedded://bevy_feathers/assets/fonts/FiraMono-Medium.ttf";
}

/// Size constants
pub mod size {
    use bevy_ui::Val;

    /// Common row size for buttons, sliders, spinners, etc.
    pub const ROW_HEIGHT: Val = Val::Px(22.0);
}
