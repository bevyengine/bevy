//! This crate contains Bevy's UI system, which can be used to create UI for both 2D and 3D games
//! # Basic usage
//! Spawn UI elements with [`node_bundles::ButtonBundle`], [`node_bundles::ImageBundle`], [`node_bundles::TextBundle`] and [`node_bundles::NodeBundle`]
//! This UI is laid out with the Flexbox paradigm (see <https://cssreference.io/flexbox/>)
mod flex;
mod focus;
mod geometry;
mod render;
mod stack;
mod ui_node;

pub mod camera_config;
pub mod node_bundles;
pub mod update;
pub mod widget;

use std::{error::Error, ops::RangeInclusive};

use bevy_input::InputSystem;
use bevy_math::remap_range;
use bevy_render::{camera::CameraUpdateSystem, extract_component::ExtractComponentPlugin};
pub use flex::*;
pub use focus::*;
pub use geometry::*;
pub use render::*;
pub use ui_node::*;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        camera_config::*, geometry::*, node_bundles::*, ui_node::*, widget::Button, Interaction,
        UiScale,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::{
    schedule::{IntoSystemDescriptor, SystemLabel},
    system::Resource,
};
use bevy_transform::TransformSystem;
use bevy_window::ModifiesWindows;
use stack::ui_stack_system;
pub use stack::UiStack;
use update::update_clipping_system;

use crate::prelude::UiCameraConfig;

/// The basic plugin for Bevy UI
#[derive(Default)]
pub struct UiPlugin;

/// The label enum labeling the types of systems in the Bevy UI
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum UiSystem {
    /// After this label, the ui flex state has been updated
    Flex,
    /// After this label, input interactions with UI entities have been updated for this frame
    Focus,
    /// After this label, the [`UiStack`] resource has been updated
    Stack,
}

/// The current scale of the UI.
///
/// A multiplier to fixed-sized ui values.
/// **Note:** This will only affect fixed ui values like [`Val::Px`]
#[derive(Debug, Resource)]
pub struct UiScale {
    /// The scale to be applied.
    pub scale: f64,
}

impl Default for UiScale {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

/// General representation of progress between two values.
#[derive(Debug)]
pub struct Progress<T>
where
    T: Send + Sync + Copy,
{
    /// The minimum value that the progress can have, inclusive.
    min: T,
    /// The maximum value that the progress can have, inlucsive.
    max: T,
    /// The current value of progress.
    value: T,
}

impl<T: Send + Sync + Copy> Progress<T>
where
    T: PartialOrd<T>,
{
    /// Creates a new progress using a value, min and max.
    ///
    /// The value must be within min and max.
    /// Will panic if value is out-of-bounds.
    pub fn new(value: T, min: T, max: T) -> Self {
        Self::from_range(value, min..=max)
    }

    /// Creates a new progress using a value and a range.
    ///
    /// The value must be within the bounds of the range.
    /// Will panic if value is out-of-bounds.
    pub fn from_range(value: T, range: RangeInclusive<T>) -> Self {
        if range.contains(&value) {
            Self {
                value,
                min: *range.start(),
                max: *range.end(),
            }
        } else {
            panic!("Tried creating progress with value out of bounds.");
        }
    }

    /// Gets the min bound of the progress.
    pub fn min(&self) -> T {
        self.min
    }

    /// Gets the max bound of the progress.
    pub fn max(&self) -> T {
        self.max
    }

    /// Gets the bounds of the progress.
    pub fn bounds(&self) -> RangeInclusive<T> {
        self.min..=self.max
    }

    /// Gets the current value of progress.
    pub fn progress(&self) -> T {
        self.value
    }

    /// Sets the progress to a new value and returns the new value if successful.
    ///
    /// Value must be between `min` and `max`.
    pub fn set_progress(&mut self, new_value: T) -> Result<T, &str> {
        if self.bounds().contains(&new_value) {
            self.value = new_value;
            Ok(self.value)
        } else {
            Err("Value outside the bounds of the Progress")
        }
    }
}

impl Progress<f32> {
    /// Returns the current progress, normalized between 0 and 1.
    /// Where 0 represents value == min,
    /// 1 represents value == max.
    pub fn progress_normalized(&self) -> f32 {
        remap_range(self.value, (self.min, self.max), (0.0, 1.0))
    }
}

impl Default for Progress<f32> {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            value: 0.0,
        }
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<UiCameraConfig>::default())
            .init_resource::<FlexSurface>()
            .init_resource::<UiScale>()
            .init_resource::<UiStack>()
            .register_type::<AlignContent>()
            .register_type::<AlignItems>()
            .register_type::<AlignSelf>()
            .register_type::<CalculatedSize>()
            .register_type::<Direction>()
            .register_type::<Display>()
            .register_type::<FlexDirection>()
            .register_type::<FlexWrap>()
            .register_type::<FocusPolicy>()
            .register_type::<Interaction>()
            .register_type::<JustifyContent>()
            .register_type::<Node>()
            // NOTE: used by Style::aspect_ratio
            .register_type::<Option<f32>>()
            .register_type::<Overflow>()
            .register_type::<PositionType>()
            .register_type::<Size>()
            .register_type::<UiRect>()
            .register_type::<Style>()
            .register_type::<BackgroundColor>()
            .register_type::<UiImage>()
            .register_type::<Val>()
            .register_type::<widget::Button>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                ui_focus_system.label(UiSystem::Focus).after(InputSystem),
            )
            // add these stages to front because these must run before transform update systems
            .add_system_to_stage(
                CoreStage::PostUpdate,
                widget::text_system
                    .before(UiSystem::Flex)
                    .after(ModifiesWindows)
                    // Potential conflict: `Assets<Image>`
                    // In practice, they run independently since `bevy_render::camera_update_system`
                    // will only ever observe its own render target, and `widget::text_system`
                    // will never modify a pre-existing `Image` asset.
                    .ambiguous_with(CameraUpdateSystem)
                    // Potential conflict: `Assets<Image>`
                    // Since both systems will only ever insert new [`Image`] assets,
                    // they will never observe each other's effects.
                    .ambiguous_with(bevy_text::update_text2d_layout),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                widget::update_image_calculated_size_system
                    .before(UiSystem::Flex)
                    // Potential conflicts: `Assets<Image>`
                    // They run independently since `widget::image_node_system` will only ever observe
                    // its own UiImage, and `widget::text_system` & `bevy_text::update_text2d_layout`
                    // will never modify a pre-existing `Image` asset.
                    .ambiguous_with(bevy_text::update_text2d_layout)
                    .ambiguous_with(widget::text_system),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                flex_node_system
                    .label(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate)
                    .after(ModifiesWindows),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                ui_stack_system.label(UiSystem::Stack),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_clipping_system.after(TransformSystem::TransformPropagate),
            );

        crate::render::build_ui_render(app);
    }
}
