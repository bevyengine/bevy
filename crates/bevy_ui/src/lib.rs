#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate contains Bevy's UI system, which can be used to create UI for both 2D and 3D games
//! # Basic usage
//! Spawn UI elements with [`widget::Button`], [`ImageNode`], [`Text`](prelude::Text) and [`Node`]
//! This UI is laid out with the Flexbox and CSS Grid layout models (see <https://cssreference.io/flexbox/>)

pub mod measurement;
pub mod node_bundles;
#[cfg(feature = "bevy_render")]
pub mod ui_material;
pub mod update;
pub mod widget;

#[cfg(feature = "bevy_ui_picking_backend")]
pub mod picking_backend;

use bevy_derive::{Deref, DerefMut};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "bevy_text", feature = "bevy_render"))]
mod accessibility;
// This module is not re-exported, but is instead made public.
// This is intended to discourage accidental use of the experimental API.
pub mod experimental;
mod focus;
mod geometry;
#[cfg(feature = "bevy_render")]
mod layout;
#[cfg(feature = "bevy_render")]
mod render;
mod stack;
mod ui_node;

pub use focus::*;
pub use geometry::*;
#[cfg(feature = "bevy_render")]
pub use layout::*;
pub use measurement::*;
#[cfg(feature = "bevy_render")]
pub use render::*;
pub use stack::UiStack;
#[cfg(feature = "bevy_render")]
pub use ui_material::*;
pub use ui_node::*;

use widget::{ImageNode, ImageNodeSize};

/// The UI prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    #[cfg(feature = "bevy_ui_debug")]
    pub use crate::render::UiDebugOptions;
    #[allow(deprecated)]
    #[doc(hidden)]
    #[cfg(feature = "bevy_text")]
    pub use crate::widget::TextBundle;
    #[doc(hidden)]
    #[cfg(feature = "bevy_text")]
    pub use crate::widget::{Text, TextUiReader, TextUiWriter};
    #[doc(hidden)]
    #[cfg(feature = "bevy_render")]
    pub use crate::{ui_material::*, MaterialNode, UiMaterialPlugin};
    #[doc(hidden)]
    pub use {
        crate::{
            geometry::*,
            node_bundles::*,
            ui_node::*,
            widget::{Button, ImageNode, Label},
            Interaction, UiScale,
        },
        // `bevy_sprite` re-exports for texture slicing
        bevy_sprite::{BorderRect, SliceScaleMode, SpriteImageMode, TextureSlicer},
    };
}

use bevy_app::{prelude::*, Animation};
use bevy_ecs::prelude::*;
use bevy_transform::TransformSystem;
use stack::ui_stack_system;
use update::update_clipping_system;

#[cfg(feature = "bevy_render")]
use {
    bevy_input::InputSystem,
    bevy_render::{camera::CameraUpdateSystem, RenderApp},
    update::update_target_camera_system,
};

/// The basic plugin for Bevy UI
pub struct UiPlugin {
    /// If set to false, the UI's rendering systems won't be added to the `RenderApp` and no UI elements will be drawn.
    /// The layout and interaction components will still be updated as normal.
    pub enable_rendering: bool,
    /// Whether to add the UI picking backend to the app.
    #[cfg(feature = "bevy_ui_picking_backend")]
    pub add_picking: bool,
}

impl Default for UiPlugin {
    fn default() -> Self {
        Self {
            enable_rendering: true,
            #[cfg(feature = "bevy_ui_picking_backend")]
            add_picking: true,
        }
    }
}

/// The label enum labeling the types of systems in the Bevy UI
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum UiSystem {
    /// After this label, input interactions with UI entities have been updated for this frame.
    ///
    /// Runs in [`PreUpdate`].
    Focus,
    /// All UI systems in [`PostUpdate`] will run in or after this label.
    Prepare,
    /// After this label, the ui layout state has been updated.
    ///
    /// Runs in [`PostUpdate`].
    Layout,
    /// UI systems ordered after [`UiSystem::Layout`].
    ///
    /// Runs in [`PostUpdate`].
    PostLayout,
    /// After this label, the [`UiStack`] resource has been updated.
    ///
    /// Runs in [`PostUpdate`].
    Stack,
}

/// The current scale of the UI.
///
/// A multiplier to fixed-sized ui values.
/// **Note:** This will only affect fixed ui values like [`Val::Px`]
#[derive(Debug, Reflect, Resource, Deref, DerefMut)]
#[reflect(Resource, Debug, Default)]
pub struct UiScale(pub f32);

impl Default for UiScale {
    fn default() -> Self {
        Self(1.0)
    }
}

// Marks systems that can be ambiguous with [`widget::text_system`] if the `bevy_text` feature is enabled.
// See https://github.com/bevyengine/bevy/pull/11391 for more details.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
#[cfg(feature = "bevy_text")]
struct AmbiguousWithTextSystem;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
#[cfg(feature = "bevy_text")]
struct AmbiguousWithUpdateText2DLayout;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_render")]
        app.init_resource::<ui_surface::UiSurface>();
        app.init_resource::<UiScale>()
            .init_resource::<UiStack>()
            .register_type::<BackgroundColor>()
            .register_type::<CalculatedClip>()
            .register_type::<ComputedNode>()
            .register_type::<ContentSize>()
            .register_type::<FocusPolicy>()
            .register_type::<Interaction>()
            .register_type::<Node>()
            .register_type::<RelativeCursorPosition>()
            .register_type::<ScrollPosition>();
        #[cfg(feature = "bevy_render")]
        app.register_type::<TargetCamera>();
        app.register_type::<ImageNode>()
            .register_type::<ImageNodeSize>()
            .register_type::<UiRect>()
            .register_type::<UiScale>()
            .register_type::<BorderColor>()
            .register_type::<BorderRadius>()
            .register_type::<BoxShadow>()
            .register_type::<widget::Button>()
            .register_type::<widget::Label>()
            .register_type::<ZIndex>()
            .register_type::<Outline>()
            .register_type::<BoxShadowSamples>()
            .register_type::<UiAntiAlias>()
            .configure_sets(
                PostUpdate,
                (
                    #[cfg(feature = "bevy_render")]
                    CameraUpdateSystem,
                    UiSystem::Prepare.before(UiSystem::Stack).after(Animation),
                    UiSystem::Layout,
                    UiSystem::PostLayout,
                )
                    .chain(),
            );
        #[cfg(feature = "bevy_render")]
        app.add_systems(
            PreUpdate,
            ui_focus_system.in_set(UiSystem::Focus).after(InputSystem),
        );

        #[cfg(feature = "bevy_render")]
        {
            let systems = ui_layout_system
                .in_set(UiSystem::Layout)
                .before(TransformSystem::TransformPropagate);
            #[cfg(feature = "bevy_text")]
            let systems = systems
                // Text and Text2D operate on disjoint sets of entities
                .ambiguous_with(bevy_text::update_text2d_layout)
                .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_text::Text2d>);
            app.add_systems(PostUpdate, systems);
        }

        let systems = ui_stack_system
            .in_set(UiSystem::Stack)
            // the systems don't care about stack index
            .ambiguous_with(update_clipping_system);
        #[cfg(feature = "bevy_render")]
        let systems = systems.ambiguous_with(ui_layout_system);
        #[cfg(feature = "bevy_text")]
        let systems = systems.in_set(AmbiguousWithTextSystem);
        #[cfg(feature = "bevy_render")]
        app.add_systems(
            PostUpdate,
            update_target_camera_system.in_set(UiSystem::Prepare),
        );
        app.add_systems(
            PostUpdate,
            (
                systems,
                update_clipping_system.after(TransformSystem::TransformPropagate),
            ),
        );

        // Potential conflicts: `Assets<Image>`
        // They run independently since `widget::image_node_system` will only ever observe
        // its own ImageNode, and `widget::text_system` & `update_text2d_layout`
        // will never modify a pre-existing `Image` asset.
        let systems = widget::update_image_content_size_system.in_set(UiSystem::Prepare);
        #[cfg(feature = "bevy_text")]
        let systems = systems
            .in_set(AmbiguousWithTextSystem)
            .in_set(AmbiguousWithUpdateText2DLayout);
        app.add_systems(PostUpdate, systems);

        #[cfg(feature = "bevy_text")]
        build_text_interop(app);

        #[cfg(feature = "bevy_ui_picking_backend")]
        if self.add_picking {
            app.add_plugins(picking_backend::UiPickingPlugin);
        }

        if !self.enable_rendering {
            return;
        }

        #[cfg(feature = "bevy_ui_debug")]
        app.init_resource::<UiDebugOptions>();

        #[cfg(feature = "bevy_render")]
        build_ui_render(app);
    }

    fn finish(&self, _app: &mut App) {
        #[cfg(feature = "bevy_render")]
        {
            let Some(render_app) = _app.get_sub_app_mut(RenderApp) else {
                return;
            };
            render_app.init_resource::<UiPipeline>();
        }
    }
}

#[cfg(feature = "bevy_text")]
fn build_text_interop(app: &mut App) {
    use crate::widget::TextNodeFlags;
    use bevy_text::*;
    use widget::Text;

    app.register_type::<TextLayoutInfo>()
        .register_type::<TextNodeFlags>()
        .register_type::<Text>();

    app.add_systems(
        PostUpdate,
        (
            (
                detect_text_needs_rerender::<Text>,
                #[cfg(feature = "bevy_render")]
                widget::measure_text_system,
            )
                .chain()
                .in_set(UiSystem::Prepare)
                // Text and Text2d are independent.
                .ambiguous_with(detect_text_needs_rerender::<Text2d>)
                // Potential conflict: `Assets<Image>`
                // Since both systems will only ever insert new [`Image`] assets,
                // they will never observe each other's effects.
                .ambiguous_with(update_text2d_layout)
                // We assume Text is on disjoint UI entities to ImageNode and UiTextureAtlasImage
                // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                .ambiguous_with(widget::update_image_content_size_system),
            #[cfg(feature = "bevy_render")]
            widget::text_system
                .in_set(UiSystem::PostLayout)
                .after(remove_dropped_font_atlas_sets)
                // Text2d and bevy_ui text are entirely on separate entities
                .ambiguous_with(detect_text_needs_rerender::<Text2d>)
                .ambiguous_with(update_text2d_layout)
                .ambiguous_with(calculate_bounds_text2d),
        ),
    );

    #[cfg(all(feature = "bevy_text", feature = "bevy_render"))]
    app.add_plugins(accessibility::AccessibilityPlugin);

    #[cfg(feature = "bevy_render")]
    app.configure_sets(
        PostUpdate,
        AmbiguousWithTextSystem.ambiguous_with(widget::text_system),
    );

    app.configure_sets(
        PostUpdate,
        AmbiguousWithUpdateText2DLayout.ambiguous_with(update_text2d_layout),
    );
}
