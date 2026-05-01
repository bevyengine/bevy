//! Widgets that provide feedback to the user, such as toasts and modals.

mod toast;

pub use toast::*;
use bevy_app::Plugin;

/// Plugin which registers all `bevy_feathers` feedback widgets.
pub struct FeedbackPlugin;

impl Plugin for FeedbackPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            ToastsPlugin,
        ));
    }
}