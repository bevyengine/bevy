use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    query::{Changed, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    template::EntityTemplate,
};
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_ui::{px, BorderRadius, Node};
use bevy_ui_widgets::{ControlOrientation, Scrollbar, ScrollbarDragState, ScrollbarThumb};

use crate::{cursor::EntityCursor, theme::ThemeBackgroundColor, tokens};

/// A scrollbar. The `target` property should point to an entity whose
/// [`ScrollPosition`](bevy_ui::ScrollPosition) will be synchronized with the scrollbar.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersScrollbarProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersScrollbar;

/// Props used to construct a [`FeathersScrollbar`] scene.
#[derive(Default, Clone)]
pub struct FeathersScrollbarProps {
    /// The entity whose scroll position will be synchronized with this scrollbar.
    pub target: EntityTemplate,
    /// Whether this is a vertical or horizontal scrollbar.
    pub orientation: ControlOrientation,
}

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct FeathersScrollbarThumb;

impl FeathersScrollbar {
    /// Scene function for scrollbar.
    pub fn scene(props: FeathersScrollbarProps) -> impl Scene {
        bsn! {
            Scrollbar {
                target: {props.target},
                orientation: {props.orientation},
                min_thumb_length: 8.0
            }
            Node {
                border_radius: BorderRadius::all(px(3))
            }
            ThemeBackgroundColor(tokens::SCROLLBAR_BG)
            Children [(
                Hovered
                ThemeBackgroundColor(tokens::SCROLLBAR_THUMB)
                ScrollbarThumb {
                    border_radius: BorderRadius::all(px(3))
                }
                FeathersScrollbarThumb
                EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            )]
        }
    }
}

fn update_scrollbar_thumb_styles(
    q_thumbs: Query<
        (Entity, &Hovered, &ThemeBackgroundColor, &ScrollbarDragState),
        (
            With<FeathersScrollbarThumb>,
            Or<(Changed<Hovered>, Changed<ScrollbarDragState>)>,
        ),
    >,
    mut commands: Commands,
) {
    for (scrollbar_ent, hovered, bg_color, drag_state) in q_thumbs.iter() {
        let bg_token = if hovered.0 || drag_state.dragging {
            tokens::SCROLLBAR_THUMB_HOVER
        } else {
            tokens::SCROLLBAR_THUMB
        };

        if bg_token != bg_color.0 {
            commands
                .entity(scrollbar_ent)
                .insert(ThemeBackgroundColor(bg_token));
        }
    }
}

/// Plugin which registers the systems for updating the scrollbar styles.
pub struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            update_scrollbar_thumb_styles.in_set(PickingSystems::Last),
        );
    }
}
