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

use bevy_app::{HierarchyPropagatePlugin, Plugin, PostUpdate, PropagateSet};
use bevy_asset::embedded_asset;
use bevy_ecs::{query::With, schedule::IntoScheduleConfigs};
use bevy_text::{TextColor, TextFont};
use bevy_ui::UiSystems;
use bevy_ui_render::UiMaterialPlugin;

use crate::{
    alpha_pattern::{AlphaPatternMaterial, AlphaPatternResource},
    controls::ControlsPlugin,
    cursor::{CursorIconPlugin, DefaultCursor, EntityCursor},
    theme::{ThemedText, UiTheme},
};

mod alpha_pattern;
pub mod constants;
pub mod controls;
pub mod cursor;
pub mod dark_theme;
pub mod font_styles;
pub mod handle_or_path;
pub mod palette;
pub mod rounded_corners;
pub mod theme;
pub mod tokens;

/// Plugin which installs observers and systems for feathers themes, cursors, and all controls.
pub struct FeathersPlugin;

impl Plugin for FeathersPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<UiTheme>();

        // Embedded font
        embedded_asset!(app, "assets/fonts/FiraSans-Bold.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-BoldItalic.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-Regular.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-Italic.ttf");
        embedded_asset!(app, "assets/fonts/FiraMono-Medium.ttf");

        // Embedded shader
        embedded_asset!(app, "assets/shaders/alpha_pattern.wgsl");

        app.add_plugins((
            ControlsPlugin,
            CursorIconPlugin,
            HierarchyPropagatePlugin::<TextColor, With<ThemedText>>::new(PostUpdate),
            HierarchyPropagatePlugin::<TextFont, With<ThemedText>>::new(PostUpdate),
            UiMaterialPlugin::<AlphaPatternMaterial>::default(),
        ));

        // This needs to run in UiSystems::Propagate so the fonts are up-to-date for `measure_text_system`
        // and `detect_text_needs_rerender` in UiSystems::Content
        app.configure_sets(
            PostUpdate,
            PropagateSet::<TextFont>::default().in_set(UiSystems::Propagate),
        );

        app.insert_resource(DefaultCursor(EntityCursor::System(
            bevy_window::SystemCursorIcon::Default,
        )));

        app.add_systems(PostUpdate, theme::update_theme)
            .add_observer(theme::on_changed_background)
            .add_observer(theme::on_changed_border)
            .add_observer(theme::on_changed_font_color)
            .add_observer(font_styles::on_changed_font);

        app.init_resource::<AlphaPatternResource>();
    }
}
