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
pub mod ui_material;
pub mod update;
pub mod widget;

#[cfg(feature = "bevy_ui_picking_backend")]
pub mod picking_backend;

use bevy_derive::{Deref, DerefMut};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
mod accessibility;
// This module is not re-exported, but is instead made public.
// This is intended to discourage accidental use of the experimental API.
pub mod experimental;
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

use widget::{ImageNode, ImageNodeSize};

/// The UI prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_ui_picking_backend")]
    #[doc(hidden)]
    pub use crate::picking_backend::{UiPickingCamera, UiPickingPlugin, UiPickingSettings};
    #[doc(hidden)]
    #[cfg(feature = "bevy_ui_debug")]
    pub use crate::render::UiDebugOptions;
    #[doc(hidden)]
    pub use crate::widget::{Text, TextUiReader, TextUiWriter};
    #[doc(hidden)]
    pub use {
        crate::{
            geometry::*,
            ui_material::*,
            ui_node::*,
            widget::{Button, ImageNode, Label, NodeImageMode},
            Interaction, MaterialNode, UiMaterialPlugin, UiScale,
        },
        // `bevy_sprite` re-exports for texture slicing
        bevy_sprite::{BorderRect, SliceScaleMode, SpriteImageMode, TextureSlicer},
    };
}

use bevy_app::{prelude::*, Animation};
use bevy_ecs::prelude::*;
use bevy_input::InputSystem;
use bevy_render::{camera::CameraUpdateSystem, RenderApp};
use bevy_transform::TransformSystem;
use layout::ui_surface::UiSurface;
use stack::ui_stack_system;
pub use stack::UiStack;
use update::{update_clipping_system, update_ui_context_system};

/// The basic plugin for Bevy UI
pub struct UiPlugin {
    /// If set to false, the UI's rendering systems won't be added to the `RenderApp` and no UI elements will be drawn.
    /// The layout and interaction components will still be updated as normal.
    pub enable_rendering: bool,
}

impl Default for UiPlugin {
    fn default() -> Self {
        Self {
            enable_rendering: true,
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
    /// Update content requirements before layout.
    Content,
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
struct AmbiguousWithTextSystem;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct AmbiguousWithUpdateText2DLayout;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiSurface>()
            .init_resource::<UiScale>()
            .init_resource::<UiStack>()
            .register_type::<BackgroundColor>()
            .register_type::<CalculatedClip>()
            .register_type::<ComputedNode>()
            .register_type::<ContentSize>()
            .register_type::<FocusPolicy>()
            .register_type::<Interaction>()
            .register_type::<Node>()
            .register_type::<RelativeCursorPosition>()
            .register_type::<ScrollPosition>()
            .register_type::<UiTargetCamera>()
            .register_type::<ImageNode>()
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
            .register_type::<TextShadow>()
            .register_type::<ComputedNodeTarget>()
            .configure_sets(
                PostUpdate,
                (
                    CameraUpdateSystem,
                    UiSystem::Prepare.after(Animation),
                    UiSystem::Content,
                    UiSystem::Layout,
                    UiSystem::PostLayout,
                )
                    .chain(),
            )
            .add_systems(
                PreUpdate,
                ui_focus_system.in_set(UiSystem::Focus).after(InputSystem),
            );

        #[cfg(feature = "bevy_ui_picking_backend")]
        app.add_plugins(picking_backend::UiPickingPlugin);

        let ui_layout_system_config = ui_layout_system
            .in_set(UiSystem::Layout)
            .before(TransformSystem::TransformPropagate);

        let ui_layout_system_config = ui_layout_system_config
            // Text and Text2D operate on disjoint sets of entities
            .ambiguous_with(bevy_text::update_text2d_layout)
            .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_text::Text2d>);

        app.add_systems(
            PostUpdate,
            (
                update_ui_context_system.in_set(UiSystem::Prepare),
                ui_layout_system_config,
                ui_stack_system
                    .in_set(UiSystem::Stack)
                    // the systems don't care about stack index
                    .ambiguous_with(update_clipping_system)
                    .ambiguous_with(ui_layout_system)
                    .in_set(AmbiguousWithTextSystem),
                update_clipping_system.after(TransformSystem::TransformPropagate),
                // Potential conflicts: `Assets<Image>`
                // They run independently since `widget::image_node_system` will only ever observe
                // its own ImageNode, and `widget::text_system` & `bevy_text::update_text2d_layout`
                // will never modify a pre-existing `Image` asset.
                widget::update_image_content_size_system
                    .in_set(UiSystem::Content)
                    .in_set(AmbiguousWithTextSystem)
                    .in_set(AmbiguousWithUpdateText2DLayout),
            ),
        );
        build_text_interop(app);

        if !self.enable_rendering {
            return;
        }

        #[cfg(feature = "bevy_ui_debug")]
        app.init_resource::<UiDebugOptions>();

        build_ui_render(app);
    }

    fn finish(&self, app: &mut App) {
        if !self.enable_rendering {
            return;
        }

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<UiPipeline>();
    }
}

fn build_text_interop(app: &mut App) {
    use crate::widget::TextNodeFlags;
    use bevy_text::TextLayoutInfo;
    use widget::Text;

    app.register_type::<TextLayoutInfo>()
        .register_type::<TextNodeFlags>()
        .register_type::<Text>();

    app.add_systems(
        PostUpdate,
        (
            (
                bevy_text::detect_text_needs_rerender::<Text>,
                widget::measure_text_system,
            )
                .chain()
                .in_set(UiSystem::Content)
                // Text and Text2d are independent.
                .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_text::Text2d>)
                // Potential conflict: `Assets<Image>`
                // Since both systems will only ever insert new [`Image`] assets,
                // they will never observe each other's effects.
                .ambiguous_with(bevy_text::update_text2d_layout)
                // We assume Text is on disjoint UI entities to ImageNode and UiTextureAtlasImage
                // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                .ambiguous_with(widget::update_image_content_size_system),
            widget::text_system
                .in_set(UiSystem::PostLayout)
                .after(bevy_text::remove_dropped_font_atlas_sets)
                .before(bevy_asset::AssetEvents)
                // Text2d and bevy_ui text are entirely on separate entities
                .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_text::Text2d>)
                .ambiguous_with(bevy_text::update_text2d_layout)
                .ambiguous_with(bevy_text::calculate_bounds_text2d),
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
