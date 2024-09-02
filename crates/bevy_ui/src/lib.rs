// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate contains Bevy's UI system, which can be used to create UI for both 2D and 3D games
//! # Basic usage
//! Spawn UI elements with [`node_bundles::ButtonBundle`], [`node_bundles::ImageBundle`], [`node_bundles::TextBundle`] and [`node_bundles::NodeBundle`]
//! This UI is laid out with the Flexbox and CSS Grid layout models (see <https://cssreference.io/flexbox/>)

pub mod measurement;
pub mod node_bundles;
pub mod ui_material;
pub mod update;
pub mod widget;

use bevy_derive::{Deref, DerefMut};
use bevy_reflect::Reflect;
#[cfg(feature = "bevy_text")]
mod accessibility;
mod focus;
mod geometry;
mod layout;
mod render;
mod stack;
mod ui_node;

pub use focus::*;
pub use geometry::*;
pub use layout::*;
pub use measurement::*;
pub use render::*;
pub use ui_material::*;
pub use ui_node::*;
use widget::UiImageSize;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        geometry::*, node_bundles::*, ui_material::*, ui_node::*, widget::Button, widget::Label,
        Interaction, UiMaterialPlugin, UiScale,
    };
    // `bevy_sprite` re-exports for texture slicing
    #[doc(hidden)]
    pub use bevy_sprite::{BorderRect, ImageScaleMode, SliceScaleMode, TextureSlicer};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::InputSystem;
use bevy_render::{
    camera::CameraUpdateSystem,
    view::{check_visibility, VisibilitySystems},
    RenderApp,
};
use bevy_transform::TransformSystem;
use layout::ui_surface::UiSurface;
use stack::ui_stack_system;
pub use stack::UiStack;
use update::{update_clipping_system, update_target_camera_system};

/// The basic plugin for Bevy UI
#[derive(Default)]
pub struct UiPlugin;

/// The label enum labeling the types of systems in the Bevy UI
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum UiSystem {
    /// After this label, the ui layout state has been updated
    Layout,
    /// After this label, input interactions with UI entities have been updated for this frame
    Focus,
    /// After this label, the [`UiStack`] resource has been updated
    Stack,
}

/// The current scale of the UI.
///
/// A multiplier to fixed-sized ui values.
/// **Note:** This will only affect fixed ui values like [`Val::Px`]
#[derive(Debug, Reflect, Resource, Deref, DerefMut)]
pub struct UiScale(pub f32);

impl Default for UiScale {
    fn default() -> Self {
        Self(1.0)
    }
}

// Marks systems that can be ambiguous with [`widget::text_system`] if the `bevy_text` feature is enabled.
// See https://github.com/bevyengine/bevy/pull/11391 for more details.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct AmbiguousWithTextSystem;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct AmbiguousWithUpdateText2DLayout;

/// A convenient alias for `With<Node>`, for use with
/// [`bevy_render::view::VisibleEntities`].
pub type WithNode = With<Node>;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiSurface>()
            .init_resource::<UiScale>()
            .init_resource::<UiStack>()
            .register_type::<BackgroundColor>()
            .register_type::<CalculatedClip>()
            .register_type::<ContentSize>()
            .register_type::<FocusPolicy>()
            .register_type::<Interaction>()
            .register_type::<Node>()
            .register_type::<RelativeCursorPosition>()
            .register_type::<Style>()
            .register_type::<TargetCamera>()
            .register_type::<UiImage>()
            .register_type::<UiImageSize>()
            .register_type::<UiRect>()
            .register_type::<UiScale>()
            .register_type::<BorderColor>()
            .register_type::<BorderRadius>()
            .register_type::<widget::Button>()
            .register_type::<widget::Label>()
            .register_type::<ZIndex>()
            .register_type::<Outline>()
            .configure_sets(
                PostUpdate,
                (CameraUpdateSystem, UiSystem::Stack, UiSystem::Layout).chain(),
            )
            .add_systems(
                PreUpdate,
                ui_focus_system.in_set(UiSystem::Focus).after(InputSystem),
            );

        app.add_systems(
            PostUpdate,
            (
                check_visibility::<WithNode>.in_set(VisibilitySystems::CheckVisibility),
                update_target_camera_system.before(UiSystem::Layout),
                apply_deferred
                    .after(update_target_camera_system)
                    .before(UiSystem::Layout),
                ui_layout_system
                    .in_set(UiSystem::Layout)
                    .before(TransformSystem::TransformPropagate),
                ui_stack_system
                    .in_set(UiSystem::Stack)
                    // the systems don't care about stack index
                    .ambiguous_with(update_clipping_system)
                    .ambiguous_with(ui_layout_system)
                    .in_set(AmbiguousWithTextSystem),
                update_clipping_system.after(TransformSystem::TransformPropagate),
                // Potential conflicts: `Assets<Image>`
                // They run independently since `widget::image_node_system` will only ever observe
                // its own UiImage, and `widget::text_system` & `bevy_text::update_text2d_layout`
                // will never modify a pre-existing `Image` asset.
                widget::update_image_content_size_system
                    .before(UiSystem::Layout)
                    .in_set(AmbiguousWithTextSystem)
                    .in_set(AmbiguousWithUpdateText2DLayout),
            ),
        );

        #[cfg(feature = "bevy_text")]
        build_text_interop(app);

        build_ui_render(app);
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<UiPipeline>();
    }
}

/// A function that should be called from [`UiPlugin::build`] when [`bevy_text`] is enabled.
#[cfg(feature = "bevy_text")]
fn build_text_interop(app: &mut App) {
    use crate::widget::TextFlags;
    use bevy_text::TextLayoutInfo;

    app.register_type::<TextLayoutInfo>()
        .register_type::<TextFlags>();

    app.add_systems(
        PostUpdate,
        (
            widget::measure_text_system
                .before(UiSystem::Layout)
                // Potential conflict: `Assets<Image>`
                // Since both systems will only ever insert new [`Image`] assets,
                // they will never observe each other's effects.
                .ambiguous_with(bevy_text::update_text2d_layout)
                // We assume Text is on disjoint UI entities to UiImage and UiTextureAtlasImage
                // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                .ambiguous_with(widget::update_image_content_size_system),
            widget::text_system
                .after(UiSystem::Layout)
                .after(bevy_text::remove_dropped_font_atlas_sets)
                // Text2d and bevy_ui text are entirely on separate entities
                .ambiguous_with(bevy_text::update_text2d_layout),
        ),
    );

    app.add_plugins(accessibility::AccessibilityPlugin);

    app.configure_sets(
        PostUpdate,
        AmbiguousWithTextSystem.ambiguous_with(widget::text_system),
    );

    app.configure_sets(
        PostUpdate,
        AmbiguousWithUpdateText2DLayout.ambiguous_with(bevy_text::update_text2d_layout),
    );
}
