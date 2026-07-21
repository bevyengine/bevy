use bevy_app::{Plugin, PostUpdate, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Changed, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    template::EntityTemplate,
};
use bevy_math::Vec2;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_ui::{px, BorderRadius, ComputedNode, Node, UiSystems, Val};
use bevy_ui_widgets::{ControlOrientation, Scrollbar, ScrollbarDragState, ScrollbarThumb};

use crate::{cursor::EntityCursor, theme::ThemeBackgroundColor, tokens};

/// A scrollbar. The `target` property should point to an entity whose
/// [`ScrollPosition`](bevy_ui::ScrollPosition) will be synchronized with the scrollbar.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersScrollbarProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersScrollbar {
    auto_hide: bool,
}

/// Props used to construct a [`FeathersScrollbar`] scene.
#[derive(Clone)]
pub struct FeathersScrollbarProps {
    /// The entity whose scroll position will be synchronized with this scrollbar.
    pub target: EntityTemplate,
    /// Whether this is a vertical or horizontal scrollbar.
    pub orientation: ControlOrientation,
    /// Auto hide if content fits
    pub auto_hide: bool,
}

impl Default for FeathersScrollbarProps {
    fn default() -> Self {
        Self {
            target: EntityTemplate::default(),
            orientation: ControlOrientation::default(),
            auto_hide: true,
        }
    }
}

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct FeathersScrollbarThumb;

/// Padding at right (vertical) and bottom (horizontal) reserved on a scrollbar's
/// parent while the scrollbar is visible. reclaimed when the content fits.
/// Do not use if scrollbars are not embedded in their parents padding on right & bottom
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ScrollbarGutter(pub Val);

impl FeathersScrollbar {
    /// Scene function for scrollbar.
    pub fn scene(props: FeathersScrollbarProps) -> impl Scene {
        bsn! {
            FeathersScrollbar {
                auto_hide: {props.auto_hide},
            }
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

fn update_scrollbar_visibility(
    mut q_scrollbars: Query<(Entity, &FeathersScrollbar, &Scrollbar, &mut Visibility)>,
    q_scroll_area: Query<&ComputedNode>,
    q_parents: Query<&ChildOf>,
    mut q_gutters: Query<(&ScrollbarGutter, &mut Node)>,
) {
    for (scrollbar_ent, feathers_scrollbar, scrollbar, mut visibility) in q_scrollbars.iter_mut() {
        if !feathers_scrollbar.auto_hide {
            continue;
        }
        let Ok(area) = q_scroll_area.get(scrollbar.target) else {
            continue;
        };
        let visible = (area.size() - area.scrollbar_size).max(Vec2::ZERO);
        let overflows = match scrollbar.orientation {
            ControlOrientation::Horizontal => area.content_size().x > visible.x,
            ControlOrientation::Vertical => area.content_size().y > visible.y,
        };
        let target = if overflows {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != target {
            *visibility = target;
        }
        if let Ok(child_of) = q_parents.get(scrollbar_ent)
            && let Ok((gutter, mut node)) = q_gutters.get_mut(child_of.parent())
        {
            let padding = if overflows { gutter.0 } else { Val::ZERO };
            match scrollbar.orientation {
                ControlOrientation::Vertical => {
                    if node.padding.right != padding {
                        node.padding.right = padding;
                    }
                }
                ControlOrientation::Horizontal => {
                    if node.padding.bottom != padding {
                        node.padding.bottom = padding;
                    }
                }
            }
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
        app.add_systems(
            PostUpdate,
            update_scrollbar_visibility.after(UiSystems::Layout),
        );
    }
}
