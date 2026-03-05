//! Various non-themable constants for the Feathers look and feel.

/// Font asset handles
pub mod fonts {
    use bevy_asset::{uuid_handle, Handle};
    use bevy_text::Font;

    /// Default regular font
    pub const REGULAR: Handle<Font> = uuid_handle!("019cbfa8-ebaf-7a1f-a798-f274b40d265b");
    /// Regular italic font
    pub const ITALIC: Handle<Font> = uuid_handle!("019cbfa8-ebaf-793c-9dbc-2870a8075f68");
    /// Bold font
    pub const BOLD: Handle<Font> = uuid_handle!("019cbfa8-ebaf-7443-b076-5ebe577652bb");
    /// Bold italic font
    pub const BOLD_ITALIC: Handle<Font> = uuid_handle!("019cbfa8-ebaf-7747-989b-385035d0eca8");
    /// Monospace font
    pub const MONO: Handle<Font> = uuid_handle!("019cbfa8-ebaf-7ce0-afc0-d21807ada543");
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
