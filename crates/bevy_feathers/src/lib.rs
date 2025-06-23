//! `bevy_feathers` is a collection of styled and themed widgets for building editors and
//! inspectors.
//! 
//! The aesthetic choices made here are designed with a future Bevy Editor in mind,
//! but this crate is deliberately exposed to the public to allow the broader ecosystem to easily create
//! tooling for themselves and others that fits cohesively together.
//!
//! While it may be tempting to use this crate for your game's UI, it's deliberately not intended for that.
//! We've opted for a clean, functional style, and prioritized consistency over customization.
//! That said, if you like what you see, it can be a helpful learning tool.
//! Consider copying this code into your own project,
//! and refining the styles and abstractions provided to meet your needs.
//! 
//! ## Warning: Experimental!
//! All that said, this crate is still experimental and unfinished!
//! It will change in breaking ways, and there will be both bugs and limitations.
//! 
//! Please report issues, submit fixes and propose changes.
//! Thanks for stress-testing; let's build something better together.

use bevy_app::{HierarchyPropagatePlugin, Plugin, PostUpdate};
use bevy_asset::embedded_asset;
use bevy_ecs::query::With;
use bevy_text::{TextColor, TextFont};
use bevy_winit::cursor::CursorIcon;

use crate::{
    controls::ControlsPlugin,
    cursor::{CursorIconPlugin, DefaultCursorIcon},
    theme::{UiTheme, UseTheme},
};

/// Standard feathers color palette.
pub mod palette;

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
        embedded_asset!(app, "assets/fonts/FiraMono-Medium.ttf");

        app.add_plugins((
            ControlsPlugin,
            CursorIconPlugin,
            HierarchyPropagatePlugin::<TextColor, With<UseTheme>>::default(),
            HierarchyPropagatePlugin::<TextFont, With<UseTheme>>::default(),
        ));

        app.insert_resource(DefaultCursorIcon(CursorIcon::System(
            bevy_window::SystemCursorIcon::Default,
        )));

        app.add_systems(PostUpdate, theme::update_theme)
            .add_observer(theme::on_changed_background)
            .add_observer(theme::on_changed_font_color)
            .add_observer(font_styles::on_changed_font);
    }
}
