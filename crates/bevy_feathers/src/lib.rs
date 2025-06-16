//! `bevy_feathers` is a collection of styled and themed widgets for building editors and
//! inspectors.

use bevy_app::{Plugin, PostUpdate};

use crate::{controls::ControlsPlugin, cursor::CursorIconPlugin};

/// Standard feathers color palette.
pub mod colors;

/// Entity cursor management.
pub mod cursor;

/// Modules containing feathers controls.
pub mod controls;

/// Module containing the default feathers dark theme.
pub mod dark;

/// Provides a way to specify an asset reference either as a handle or as an asset path.
pub mod handle_or_path;

/// Inheritable text styles.
pub mod font_styles;

/// Module containing the themeing framework used by feathers.
pub mod theme;

/// Plugin which installs observers and systems for feathers themes, cursors, and all controls.
pub struct FeathersPlugin;

impl Plugin for FeathersPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((ControlsPlugin, CursorIconPlugin));
        app.add_systems(PostUpdate, font_styles::update_text_styles);
        app.add_observer(font_styles::set_initial_text_style);
    }
}
