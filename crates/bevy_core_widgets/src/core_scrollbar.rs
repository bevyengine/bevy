use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{With, Without},
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_math::Vec2;
use bevy_picking::events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    ComputedNode, ComputedUiTargetCamera, Node, ScrollPosition, UiGlobalTransform, UiScale, Val,
};

/// Used to select the orientation of a scrollbar, slider, or other oriented control.
// TODO: Move this to a more central place.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(PartialEq, Clone, Default)]
pub enum ControlOrientation {
    /// Horizontal orientation (stretching from left to right)
    Horizontal,
    /// Vertical orientation (stretching from top to bottom)
    #[default]
    Vertical,
}

/// A headless scrollbar widget, which can be used to build custom scrollbars.
///
/// Scrollbars operate differently than the other core widgets in a number of respects.
///
/// Unlike sliders, scrollbars don't have an [`AccessibilityNode`](bevy_a11y::AccessibilityNode)
/// component, nor can they have keyboard focus. This is because scrollbars are usually used in
/// conjunction with a scrollable container, which is itself accessible and focusable. This also
/// means that scrollbars don't accept keyboard events, which is also the responsibility of the
/// scrollable container.
///
/// Scrollbars don't emit notification events; instead they modify the scroll position of the target
/// entity directly.
///
/// A scrollbar can have any number of child entities, but one entity must be the scrollbar thumb,
/// which is marked with the [`CoreScrollbarThumb`] component. Other children are ignored. The core
/// scrollbar will directly update the position and size of this entity; the application is free to
/// set any other style properties as desired.
///
/// The application is free to position the scrollbars relative to the scrolling container however
/// it wants: it can overlay them on top of the scrolling content, or use a grid layout to displace
/// the content to make room for the scrollbars.
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct CoreScrollbar {
    /// Entity being scrolled.
    pub target: Entity,
    /// Whether the scrollbar is vertical or horizontal.
    pub orientation: ControlOrientation,
    /// Minimum length of the scrollbar thumb, in pixel units, in the direction parallel to the main
    /// scrollbar axis. The scrollbar will resize the thumb entity based on the proportion of
    /// visible size to content size, but no smaller than this. This prevents the thumb from
    /// disappearing in cases where the ratio of content size to visible size is large.
    pub min_thumb_length: f32,
}

/// Marker component to indicate that the entity is a scrollbar thumb (the moving, draggable part of
/// the scrollbar). This should be a child of the scrollbar entity.
#[derive(Component, Debug)]
#[require(CoreScrollbarDragState)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct CoreScrollbarThumb;

impl CoreScrollbar {
    /// Construct a new scrollbar.
    ///
    /// # Arguments
    ///
    /// * `target` - The scrollable entity that this scrollbar will control.
    /// * `orientation` - The orientation of the scrollbar (horizontal or vertical).
    /// * `min_thumb_length` - The minimum size of the scrollbar's thumb, in pixels.
    pub fn new(target: Entity, orientation: ControlOrientation, min_thumb_length: f32) -> Self {
        Self {
            target,
            orientation,
            min_thumb_length,
        }
    }
}

/// Component used to manage the state of a scrollbar during dragging. This component is
/// inserted on the thumb entity.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CoreScrollbarDragState {
    /// Whether the scrollbar is currently being dragged.
    pub dragging: bool,
    /// The value of the scrollbar when dragging started.
    drag_origin: f32,
}

fn scrollbar_on_pointer_down(
    mut ev: On<Pointer<Press>>,
    q_thumb: Query<&ChildOf, With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(
        &CoreScrollbar,
        &ComputedNode,
        &ComputedUiTargetCamera,
        &UiGlobalTransform,
    )>,
    mut q_scroll_pos: Query<(&mut ScrollPosition, &ComputedNode), Without<CoreScrollbar>>,
    ui_scale: Res<UiScale>,
) {
    if q_thumb.contains(ev.target()) {
        // If they click on the thumb, do nothing. This will be handled by the drag event.
        ev.propagate(false);
    } else if let Ok((scrollbar, node, node_target, transform)) = q_scrollbar.get_mut(ev.target()) {
        // If they click on the scrollbar track, page up or down.
        ev.propagate(false);

        // Convert to widget-local coordinates.
        let local_pos = transform.try_inverse().unwrap().transform_point2(
            ev.event().pointer_location.position * node_target.scale_factor() / ui_scale.0,
        ) + node.size() * 0.5;

        // Bail if we don't find the target entity.
        let Ok((mut scroll_pos, scroll_content)) = q_scroll_pos.get_mut(scrollbar.target) else {
            return;
        };

        // Convert the click coordinates into a scroll position. If it's greater than the
        // current scroll position, scroll forward by one step (visible size) otherwise scroll
        // back.
        let visible_size = scroll_content.size() * scroll_content.inverse_scale_factor;
        let content_size = scroll_content.content_size() * scroll_content.inverse_scale_factor;
        let max_range = (content_size - visible_size).max(Vec2::ZERO);

        fn adjust_scroll_pos(scroll_pos: &mut f32, click_pos: f32, step: f32, range: f32) {
            *scroll_pos =
                (*scroll_pos + if click_pos > *scroll_pos { step } else { -step }).clamp(0., range);
        }

        match scrollbar.orientation {
            ControlOrientation::Horizontal => {
                if node.size().x > 0. {
                    let click_pos = local_pos.x * content_size.x / node.size().x;
                    adjust_scroll_pos(&mut scroll_pos.x, click_pos, visible_size.x, max_range.x);
                }
            }
            ControlOrientation::Vertical => {
                if node.size().y > 0. {
                    let click_pos = local_pos.y * content_size.y / node.size().y;
                    adjust_scroll_pos(&mut scroll_pos.y, click_pos, visible_size.y, max_range.y);
                }
            }
        }
    }
}

fn scrollbar_on_drag_start(
    mut ev: On<Pointer<DragStart>>,
    mut q_thumb: Query<(&ChildOf, &mut CoreScrollbarDragState), With<CoreScrollbarThumb>>,
    q_scrollbar: Query<&CoreScrollbar>,
    q_scroll_area: Query<&ScrollPosition>,
) {
    if let Ok((ChildOf(thumb_parent), mut drag)) = q_thumb.get_mut(ev.target()) {
        ev.propagate(false);
        if let Ok(scrollbar) = q_scrollbar.get(*thumb_parent)
            && let Ok(scroll_area) = q_scroll_area.get(scrollbar.target)
        {
            drag.dragging = true;
            drag.drag_origin = match scrollbar.orientation {
                ControlOrientation::Horizontal => scroll_area.x,
                ControlOrientation::Vertical => scroll_area.y,
            };
        }
    }
}

fn scrollbar_on_drag(
    mut ev: On<Pointer<Drag>>,
    mut q_thumb: Query<(&ChildOf, &mut CoreScrollbarDragState), With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(&ComputedNode, &CoreScrollbar)>,
    mut q_scroll_pos: Query<(&mut ScrollPosition, &ComputedNode), Without<CoreScrollbar>>,
    ui_scale: Res<UiScale>,
) {
    if let Ok((ChildOf(thumb_parent), drag)) = q_thumb.get_mut(ev.target())
        && let Ok((node, scrollbar)) = q_scrollbar.get_mut(*thumb_parent)
    {
        ev.propagate(false);
        let Ok((mut scroll_pos, scroll_content)) = q_scroll_pos.get_mut(scrollbar.target) else {
            return;
        };

        if drag.dragging {
            let distance = ev.event().distance / ui_scale.0;
            let visible_size = scroll_content.size() * scroll_content.inverse_scale_factor;
            let content_size = scroll_content.content_size() * scroll_content.inverse_scale_factor;
            let scrollbar_size = (node.size() * node.inverse_scale_factor).max(Vec2::ONE);

            match scrollbar.orientation {
                ControlOrientation::Horizontal => {
                    let range = (content_size.x - visible_size.x).max(0.);
                    scroll_pos.x = (drag.drag_origin
                        + (distance.x * content_size.x) / scrollbar_size.x)
                        .clamp(0., range);
                }
                ControlOrientation::Vertical => {
                    let range = (content_size.y - visible_size.y).max(0.);
                    scroll_pos.y = (drag.drag_origin
                        + (distance.y * content_size.y) / scrollbar_size.y)
                        .clamp(0., range);
                }
            };
        }
    }
}

fn scrollbar_on_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    mut q_thumb: Query<&mut CoreScrollbarDragState, With<CoreScrollbarThumb>>,
) {
    if let Ok(mut drag) = q_thumb.get_mut(ev.target()) {
        ev.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn scrollbar_on_drag_cancel(
    mut ev: On<Pointer<Cancel>>,
    mut q_thumb: Query<&mut CoreScrollbarDragState, With<CoreScrollbarThumb>>,
) {
    if let Ok(mut drag) = q_thumb.get_mut(ev.target()) {
        ev.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn update_scrollbar_thumb(
    q_scroll_area: Query<(&ScrollPosition, &ComputedNode)>,
    q_scrollbar: Query<(&CoreScrollbar, &ComputedNode, &Children)>,
    mut q_thumb: Query<&mut Node, With<CoreScrollbarThumb>>,
) {
    for (scrollbar, scrollbar_node, children) in q_scrollbar.iter() {
        let Ok(scroll_area) = q_scroll_area.get(scrollbar.target) else {
            continue;
        };

        // Size of the visible scrolling area.
        let visible_size = scroll_area.1.size() * scroll_area.1.inverse_scale_factor;

        // Size of the scrolling content.
        let content_size = scroll_area.1.content_size() * scroll_area.1.inverse_scale_factor;

        // Length of the scrollbar track.
        let track_length = scrollbar_node.size() * scrollbar_node.inverse_scale_factor;

        fn size_and_pos(
            content_size: f32,
            visible_size: f32,
            track_length: f32,
            min_size: f32,
            offset: f32,
        ) -> (f32, f32) {
            let thumb_size = if content_size > visible_size {
                (track_length * visible_size / content_size)
                    .max(min_size)
                    .min(track_length)
            } else {
                track_length
            };

            let thumb_pos = if content_size > visible_size {
                offset * (track_length - thumb_size) / (content_size - visible_size)
            } else {
                0.
            };

            (thumb_size, thumb_pos)
        }

        for child in children {
            if let Ok(mut thumb) = q_thumb.get_mut(*child) {
                match scrollbar.orientation {
                    ControlOrientation::Horizontal => {
                        let (thumb_size, thumb_pos) = size_and_pos(
                            content_size.x,
                            visible_size.x,
                            track_length.x,
                            scrollbar.min_thumb_length,
                            scroll_area.0.x,
                        );

                        thumb.top = Val::Px(0.);
                        thumb.bottom = Val::Px(0.);
                        thumb.left = Val::Px(thumb_pos);
                        thumb.width = Val::Px(thumb_size);
                    }
                    ControlOrientation::Vertical => {
                        let (thumb_size, thumb_pos) = size_and_pos(
                            content_size.y,
                            visible_size.y,
                            track_length.y,
                            scrollbar.min_thumb_length,
                            scroll_area.0.y,
                        );

                        thumb.left = Val::Px(0.);
                        thumb.right = Val::Px(0.);
                        thumb.top = Val::Px(thumb_pos);
                        thumb.height = Val::Px(thumb_size);
                    }
                };
            }
        }
    }
}

/// Plugin that adds the observers for the [`CoreScrollbar`] widget.
pub struct CoreScrollbarPlugin;

impl Plugin for CoreScrollbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scrollbar_on_pointer_down)
            .add_observer(scrollbar_on_drag_start)
            .add_observer(scrollbar_on_drag_end)
            .add_observer(scrollbar_on_drag_cancel)
            .add_observer(scrollbar_on_drag)
            .add_systems(PostUpdate, update_scrollbar_thumb);
    }
}
