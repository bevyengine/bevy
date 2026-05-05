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

/// Icon paths
pub mod icons {
    /// Downward-pointing chevron
    pub const CHEVRON_DOWN: &str = "embedded://bevy_feathers/assets/icons/chevron-down.png";
    /// Right-pointing chevron
    pub const CHEVRON_RIGHT: &str = "embedded://bevy_feathers/assets/icons/chevron-right.png";
    /// Diagonal Cross
    pub const X: &str = "embedded://bevy_feathers/assets/icons/x.png";
}

/// Size constants
pub mod size {
    use bevy_text::FontSize;
    use bevy_ui::Val;

    /// Common row size for buttons, sliders, spinners, etc.
    pub const ROW_HEIGHT: Val = Val::Px(24.0);

    /// Width and height of a checkbox
    pub const CHECKBOX_SIZE: Val = Val::Px(18.0);

    /// Height for pane headers
    pub const HEADER_HEIGHT: Val = Val::Px(30.0);

    /// Width and height of a radio button
    pub const RADIO_SIZE: Val = Val::Px(18.0);

    /// Width of a toggle switch
    pub const TOGGLE_WIDTH: Val = Val::Px(32.0);

    /// Height of a toggle switch
    pub const TOGGLE_HEIGHT: Val = Val::Px(18.0);

    /// Regular font size, used for most widget captions
    pub const MEDIUM_FONT: FontSize = FontSize::Px(14.0);

    /// Slightly smaller font size, used for text inputs
    pub const COMPACT_FONT: FontSize = FontSize::Px(13.0);

    /// Small font size
    pub const SMALL_FONT: FontSize = FontSize::Px(12.0);

    /// Extra-small font size
    pub const EXTRA_SMALL_FONT: FontSize = FontSize::Px(11.0);
}
