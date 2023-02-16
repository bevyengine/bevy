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
use bevy_ecs::prelude::*;
use bevy_input::InputSystem;
use bevy_transform::TransformSystem;
use stack::ui_stack_system;
pub use stack::UiStack;
use update::update_clipping_system;

use crate::prelude::UiCameraConfig;

/// The basic plugin for Bevy UI
#[derive(Default)]
pub struct UiPlugin;

/// The label enum labeling the types of systems in the Bevy UI
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
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
            .configure_set(UiSystem::Focus.in_base_set(CoreSet::PreUpdate))
            .configure_set(UiSystem::Flex.in_base_set(CoreSet::PostUpdate))
            .configure_set(UiSystem::Stack.in_base_set(CoreSet::PostUpdate))
            .add_system(ui_focus_system.in_set(UiSystem::Focus).after(InputSystem))
            // add these systems to front because these must run before transform update systems
            .add_system(
                widget::text_system
                    .in_base_set(CoreSet::PostUpdate)
                    .before(UiSystem::Flex)
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
            .add_system(
                widget::update_image_calculated_size_system
                    .in_base_set(CoreSet::PostUpdate)
                    .before(UiSystem::Flex)
                    // Potential conflicts: `Assets<Image>`
                    // They run independently since `widget::image_node_system` will only ever observe
                    // its own UiImage, and `widget::text_system` & `bevy_text::update_text2d_layout`
                    // will never modify a pre-existing `Image` asset.
                    .ambiguous_with(bevy_text::update_text2d_layout)
                    .ambiguous_with(widget::text_system),
            )
            .add_system(
                flex_node_system
                    .in_set(UiSystem::Flex)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system(ui_stack_system.in_set(UiSystem::Stack))
            .add_system(
                update_clipping_system
                    .after(TransformSystem::TransformPropagate)
                    .in_base_set(CoreSet::PostUpdate),
            );

        crate::render::build_ui_render(app);
    }
}
