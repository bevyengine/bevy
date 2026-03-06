//! Various non-themable constants for the Feathers look and feel.

/// Font sources
pub mod fonts {
    use bevy_text::FontSource;
    use smol_str::SmolStr;

    /// Default regular font
    pub const REGULAR: FontSource =
        new_static("embedded://bevy_feathers/assets/fonts/FiraSans-Regular.ttf");
    /// Regular italic font
    pub const ITALIC: FontSource =
        new_static("embedded://bevy_feathers/assets/fonts/FiraSans-Italic.ttf");
    /// Bold font
    pub const BOLD: FontSource =
        new_static("embedded://bevy_feathers/assets/fonts/FiraSans-Bold.ttf");
    /// Bold italic font
    pub const BOLD_ITALIC: FontSource =
        new_static("embedded://bevy_feathers/assets/fonts/FiraSans-BoldItalic.ttf");
    /// Monospace font
    pub const MONO: FontSource =
        new_static("embedded://bevy_feathers/assets/fonts/FiraMono-Medium.ttf");

    const fn new_static(path: &'static str) -> FontSource {
        FontSource::Family(SmolStr::new_static(path))
    }
}

/// Size constants
pub mod size {
    use bevy_ui::Val;

    /// Common row size for buttons, sliders, spinners, etc.
    pub const ROW_HEIGHT: Val = Val::Px(24.0);

    /// Width and height of a checkbox
    pub const CHECKBOX_SIZE: Val = Val::Px(18.0);

    /// Width and height of a radio button
    pub const RADIO_SIZE: Val = Val::Px(18.0);

    /// Width of a toggle switch
    pub const TOGGLE_WIDTH: Val = Val::Px(32.0);

    /// Height of a toggle switch
    pub const TOGGLE_HEIGHT: Val = Val::Px(18.0);
}
