#![allow(clippy::type_complexity)]

//! This crate contains Bevy's UI system, which can be used to create UI for both 2D and 3D games
//! # Basic usage
//! Spawn UI elements with [`node_bundles::ButtonBundle`], [`node_bundles::ImageBundle`], [`node_bundles::TextBundle`] and [`node_bundles::NodeBundle`]
//! This UI is laid out with the Flexbox layout model (see <https://cssreference.io/flexbox/>)
mod flex;
mod focus;
mod geometry;
mod render;
mod stack;
mod ui_node;

#[cfg(feature = "bevy_text")]
mod accessibility;
pub mod camera_config;
pub mod node_bundles;
pub mod update;
pub mod widget;

#[cfg(feature = "bevy_text")]
use bevy_render::camera::CameraUpdateSystem;
use bevy_render::extract_component::ExtractComponentPlugin;
pub use flex::*;
pub use focus::*;
pub use geometry::*;
pub use render::*;
pub use ui_node::*;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        camera_config::*, flex::Node, geometry::*, node_bundles::*, ui_node::*, widget::*,
        Interaction, UiScale,
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
    /// After this label, window events have been handled and windows have been associated to taffy nodes
    Windows,
    /// After this label, removed ui nodes have been removed from the layout tree
    Removal,
    /// After this label, new ui nodes have been added to the layout tree
    Insertion,
    /// After this label, the Bevy and Taffy Parent-Children trees have been synchonised
    Children,
    /// After this label, the ui flex state has been updated
    Flex,
    /// After this label, the ui node transforms have been updated
    Transforms,
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
            .init_resource::<UiSurface>()
            .init_resource::<UiScale>()
            .init_resource::<UiStack>()
            .init_resource::<UiContext>()
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
            .register_type::<NodeSize>()
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
            .register_type::<widget::Label>()
            .register_type::<Node>()
            .add_systems(
                PreUpdate,
                ui_focus_system.in_set(UiSystem::Focus).after(InputSystem),
            );
        // add these systems to front because these must run before transform update systems
        #[cfg(feature = "bevy_text")]
        app.add_systems(
            PostUpdate,
            widget::text_system
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
        );
        #[cfg(feature = "bevy_text")]
        app.add_plugin(accessibility::AccessibilityPlugin);
        app.add_systems(PostUpdate, {
            let system = widget::update_image_calculated_size_system.before(UiSystem::Flex);
            // Potential conflicts: `Assets<Image>`
            // They run independently since `widget::image_node_system` will only ever observe
            // its own UiImage, and `widget::text_system` & `bevy_text::update_text2d_layout`
            // will never modify a pre-existing `Image` asset.
            #[cfg(feature = "bevy_text")]
            let system = system
                .ambiguous_with(bevy_text::update_text2d_layout)
                .ambiguous_with(widget::text_system);

            system
        })
        .add_systems(
            PostUpdate,
            (
                update_ui_windows
                    .in_set(UiSystem::Windows)
                    .before(UiSystem::Flex),
                clean_up_removed_ui_nodes
                    .in_set(UiSystem::Removal)
                    .before(UiSystem::Insertion),
                insert_new_ui_nodes
                    .in_set(UiSystem::Insertion)
                    .before(UiSystem::Children),
                synchonise_ui_children
                    .in_set(UiSystem::Children)
                    .before(UiSystem::Flex),
                update_ui_layout
                    .in_set(UiSystem::Flex)
                    .before(UiSystem::Transforms),
                update_ui_node_transforms
                    .in_set(UiSystem::Transforms)
                    .before(TransformSystem::TransformPropagate),
                ui_stack_system.in_set(UiSystem::Stack),
                update_clipping_system.after(TransformSystem::TransformPropagate),
            ),
        );

        crate::render::build_ui_render(app);
    }
}
