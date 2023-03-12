#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! The official collection of user interface widgets for `bevy_ui`.

mod button;
mod image;
mod label;
#[cfg(feature = "bevy_text")]
mod text;

use bevy_ecs::system::Query;
use bevy_hierarchy::Children;
use bevy_text::Text;
pub use button::*;
pub use image::*;
pub use label::*;
#[cfg(feature = "bevy_text")]
pub use text::*;

use bevy_app::{App, Plugin};

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use super::{Button, ButtonBundle, ImageBundle};

    #[cfg(feature = "bevy_text")]
    #[doc(hidden)]
    pub use super::TextBundle;
}

/// Calculate the name of a widget for accessibility purposes
fn calc_name(texts: &Query<&Text>, children: &Children) -> Option<Box<str>> {
    let mut name = None;
    for child in children.iter() {
        if let Ok(text) = texts.get(*child) {
            let values = text
                .sections
                .iter()
                .map(|v| v.value.to_string())
                .collect::<Vec<String>>();
            name = Some(values.join(" "));
        }
    }
    name.map(|v| v.into_boxed_str())
}

/// The plugin for UI widgets
#[derive(Default)]
pub struct WidgetPlugin;

impl Plugin for WidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ButtonPlugin)
            .add_plugin(ImagePlugin)
            .add_plugin(LabelPlugin);

        #[cfg(feature = "bevy_text")]
        app.add_plugin(TextPlugin)
    }
}
