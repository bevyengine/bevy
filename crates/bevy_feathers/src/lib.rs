//! `bevy_feathers` is a collection of styled and themed widgets for building editors and
//! inspectors.

use bevy_app::{HierarchyPropagatePlugin, Plugin, PostUpdate};
use bevy_asset::embedded_asset;
use bevy_ecs::query::With;
use bevy_text::{TextColor, TextFont};

use crate::{
    controls::ControlsPlugin,
    cursor::CursorIconPlugin,
    theme::{UiTheme, UseTheme},
};

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
        app.init_resource::<UiTheme>();

        embedded_asset!(app, "assets/fonts/FiraSans-Bold.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-BoldItalic.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-Regular.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-Italic.ttf");

        app.add_plugins((
            ControlsPlugin,
            CursorIconPlugin,
            HierarchyPropagatePlugin::<TextColor, With<UseTheme>>::default(),
            HierarchyPropagatePlugin::<TextFont, With<UseTheme>>::default(),
        ));
        app.add_systems(PostUpdate, theme::update_theme);
        app.add_observer(theme::on_changed_background);
        app.add_observer(theme::on_changed_font_color);
        app.add_observer(font_styles::on_changed_font);
    }
}
