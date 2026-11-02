use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Has, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_picking::{cursor::EntityCursor, hover::Hovered, Pickable};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_ui::{
    px, AlignItems, Display, FlexDirection, InteractionDisabled, JustifyContent, Node, Overflow,
    UiSystems,
};
use bevy_ui_widgets::{ControlOrientation, Pane, SplitPane, SplitPaneDragState, SplitPaneHandle};
use bevy_window::SystemCursorIcon;

use crate::{theme::ThemeBackgroundColor, tokens};

const HANDLE_SIZE: f32 = 5.0;
const HANDLE_LINE_SIZE: f32 = 1.0;

/// A container that divides its space between [`FeathersPane`] children, separated by
/// [`FeathersSplitPaneHandle`]s.
///
/// A more complete explanation of how to control this widget can be found in the documentation
/// for [`SplitPane`] and [`bevy_ui_widgets`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersSplitPaneProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersSplitPane;

/// Props used to construct a [`FeathersSplitPane`] scene.
#[derive(Clone)]
pub struct FeathersSplitPaneProps {
    /// The axis along which panes are laid out.
    pub orientation: ControlOrientation,
}

impl Default for FeathersSplitPaneProps {
    fn default() -> Self {
        Self {
            orientation: ControlOrientation::Horizontal,
        }
    }
}

impl FeathersSplitPane {
    /// Scene function for split pane.
    pub fn scene(props: FeathersSplitPaneProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                align_items: AlignItems::Stretch,
            }
            SplitPane {
                orientation: {props.orientation},
            }
        }
    }
}

/// A resizable child of a [`FeathersSplitPane`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersPaneProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersPane;

/// Props used to construct a [`FeathersPane`] scene.
#[derive(Clone)]
pub struct FeathersPaneProps {
    /// The flex weight of this pane along the split axis.
    pub size: f32,
    /// The minimum size along the split axis, in logical pixels.
    pub min_size: f32,
}

impl Default for FeathersPaneProps {
    fn default() -> Self {
        Self {
            size: 1.0,
            min_size: 0.0,
        }
    }
}

impl FeathersPane {
    /// Scene function for a split pane child.
    pub fn scene(props: FeathersPaneProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                overflow: Overflow::clip(),
            }
            Pane {
                size: {props.size},
                min_size: {props.min_size},
            }
        }
    }
}

/// A draggable divider between two [`FeathersPane`]s, drawn as a thin line.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersSplitPaneHandle;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct FeathersSplitPaneHandleLine;

impl FeathersSplitPaneHandle {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                flex_basis: px(HANDLE_SIZE),
                flex_shrink: 0.0,
            }
            SplitPaneHandle
            Hovered
            Children [
                FeathersSplitPaneHandleLine
                Node {
                    flex_basis: px(HANDLE_LINE_SIZE),
                }
                ThemeBackgroundColor(tokens::SPLIT_PANE_HANDLE)
                Pickable::IGNORE
            ]
        }
    }
}

fn update_split_pane_handle_styles(
    mut q_handles: Query<
        (
            Entity,
            &ChildOf,
            &Children,
            &Hovered,
            &SplitPaneDragState,
            Has<InteractionDisabled>,
            &mut Node,
            Option<&EntityCursor>,
        ),
        With<FeathersSplitPaneHandle>,
    >,
    q_split: Query<(&SplitPane, Has<InteractionDisabled>)>,
    q_lines: Query<&ThemeBackgroundColor, With<FeathersSplitPaneHandleLine>>,
    mut commands: Commands,
) {
    for (handle, child_of, children, hovered, drag_state, handle_disabled, mut node, cursor) in
        q_handles.iter_mut()
    {
        let Ok((split, split_disabled)) = q_split.get(child_of.parent()) else {
            continue;
        };
        let disabled = handle_disabled || split_disabled;

        let (direction, resize_icon) = match split.orientation {
            ControlOrientation::Horizontal => (FlexDirection::Row, SystemCursorIcon::ColResize),
            ControlOrientation::Vertical => (FlexDirection::Column, SystemCursorIcon::RowResize),
        };
        if node.flex_direction != direction {
            node.flex_direction = direction;
        }

        let cursor_shape = EntityCursor::System(match disabled {
            true => SystemCursorIcon::NotAllowed,
            false => resize_icon,
        });
        if cursor != Some(&cursor_shape) {
            commands.entity(handle).insert(cursor_shape);
        }

        let line_token = match (disabled, drag_state.dragging, hovered.0) {
            (true, _, _) => tokens::SPLIT_PANE_HANDLE_DISABLED,
            (false, true, _) => tokens::SPLIT_PANE_HANDLE_PRESSED,
            (false, false, true) => tokens::SPLIT_PANE_HANDLE_HOVER,
            (false, false, false) => tokens::SPLIT_PANE_HANDLE,
        };
        for &line in children {
            if let Ok(bg_color) = q_lines.get(line)
                && bg_color.0 != line_token
            {
                commands
                    .entity(line)
                    .insert(ThemeBackgroundColor(line_token.clone()));
            }
        }
    }
}

/// Plugin which registers the systems for updating the split pane styles.
pub struct SplitPanePlugin;

impl Plugin for SplitPanePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            update_split_pane_handle_styles.in_set(UiSystems::Content),
        );
    }
}
