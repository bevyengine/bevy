use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{With, Without},
    system::{Query, Res},
};
use bevy_math::Vec2;
use bevy_picking::events::{Drag, DragEnd, DragStart, Pointer, Press};
use bevy_ui::{
    ComputedNode, ComputedNodeTarget, Node, ScrollPosition, UiGlobalTransform, UiScale, Val,
};

/// Used to select the orientation of the scrollbar.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Orientation {
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
/// Unlike sliders, scrollbars don't have an `AccessibilityNode` component, nor can they have
/// keyboard focus. This is because scrollbars are usually used in conjunction with a scrollable
/// container, which is itself accessible and focusable. This also means that scrollbars don't
/// accept keyboard events, which is also the responsibility of the scrollable container.
///
/// Scrollbars don't emit notification events; instead they modify the scroll position of the
/// target entity directly.
///
/// A scrollbar can have any number of child entities, but one entity must be the scrollbar
/// thumb, which is marked with the [`CoreScrollbarThumb`] component. Other children are ignored.
/// The core scrollbar will directly update the position and size of this entity; the application
/// is free to set any other style properties as desired.
///
/// The application is free to position the scrollbars relative to the scrolling container however
/// it wants: it can overlay them on top of the scrolling content, or use a grid layout to displace
/// the content to make room for the scrollbars.
#[derive(Component, Debug)]
#[require(ScrollbarDragState)]
pub struct CoreScrollbar {
    /// Entity being scrolled.
    pub target: Entity,
    /// Whether the scrollbar is vertical or horizontal.
    pub orientation: Orientation,
    /// Minimum size of the scrollbar thumb, in pixel units.
    pub min_thumb_size: f32,
}

/// Marker component to indicate that the entity is a scrollbar thumb. This should be a child
/// of the scrollbar entity.
#[derive(Component, Debug)]
pub struct CoreScrollbarThumb;

impl CoreScrollbar {
    /// Construct a new scrollbar.
    ///
    /// # Arguments
    ///
    /// * `target` - The scrollable entity that this scrollbar will control.
    /// * `orientation` - The orientation of the scrollbar (horizontal or vertical).
    /// * `min_thumb_size` - The minimum size of the scrollbar's thumb, in pixels.
    pub fn new(target: Entity, orientation: Orientation, min_thumb_size: f32) -> Self {
        Self {
            target,
            orientation,
            min_thumb_size,
        }
    }
}

/// Component used to manage the state of a scrollbar during dragging.
#[derive(Component, Default)]
pub struct ScrollbarDragState {
    /// Whether the scrollbar is currently being dragged.
    dragging: bool,
    /// The value of the scrollbar when dragging started.
    offset: f32,
}

fn scrollbar_on_pointer_down(
    mut ev: On<Pointer<Press>>,
    q_thumb: Query<&ChildOf, With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(
        &CoreScrollbar,
        &ComputedNode,
        &ComputedNodeTarget,
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
        match scrollbar.orientation {
            Orientation::Horizontal => {
                if node.size().x > 0. {
                    let click_pos = local_pos.x * content_size.x / node.size().x;
                    scroll_pos.offset_x = (scroll_pos.offset_x
                        + if click_pos > scroll_pos.offset_x {
                            visible_size.x
                        } else {
                            -visible_size.x
                        })
                    .clamp(0., max_range.x);
                }
            }
            Orientation::Vertical => {
                if node.size().y > 0. {
                    let click_pos = local_pos.y * content_size.y / node.size().y;
                    scroll_pos.offset_y = (scroll_pos.offset_y
                        + if click_pos > scroll_pos.offset_y {
                            visible_size.y
                        } else {
                            -visible_size.y
                        })
                    .clamp(0., max_range.y);
                }
            }
        }
    }
}

fn scrollbar_on_drag_start(
    mut ev: On<Pointer<DragStart>>,
    q_thumb: Query<&ChildOf, With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(&CoreScrollbar, &mut ScrollbarDragState)>,
    q_scroll_area: Query<&ScrollPosition>,
) {
    if let Ok(ChildOf(thumb_parent)) = q_thumb.get(ev.target()) {
        ev.propagate(false);
        if let Ok((scrollbar, mut drag)) = q_scrollbar.get_mut(*thumb_parent) {
            if let Ok(scroll_area) = q_scroll_area.get(scrollbar.target) {
                drag.dragging = true;
                drag.offset = match scrollbar.orientation {
                    Orientation::Horizontal => scroll_area.offset_x,
                    Orientation::Vertical => scroll_area.offset_y,
                };
            }
        }
    }
}

fn scrollbar_on_drag(
    mut ev: On<Pointer<Drag>>,
    mut q_scrollbar: Query<(&ComputedNode, &CoreScrollbar, &mut ScrollbarDragState)>,
    mut q_scroll_pos: Query<(&mut ScrollPosition, &ComputedNode), Without<CoreScrollbar>>,
) {
    if let Ok((node, scrollbar, drag)) = q_scrollbar.get_mut(ev.target()) {
        ev.propagate(false);
        let Ok((mut scroll_pos, scroll_content)) = q_scroll_pos.get_mut(scrollbar.target) else {
            return;
        };

        if drag.dragging {
            let distance = ev.event().distance;
            let visible_size = scroll_content.size() * scroll_content.inverse_scale_factor;
            let content_size = scroll_content.content_size() * scroll_content.inverse_scale_factor;
            match scrollbar.orientation {
                Orientation::Horizontal => {
                    let range = (content_size.x - visible_size.x).max(0.);
                    let scrollbar_width = (node.size().x * node.inverse_scale_factor
                        - scrollbar.min_thumb_size)
                        .max(1.0);
                    scroll_pos.offset_x = if range > 0. {
                        (drag.offset + (distance.x * content_size.x) / scrollbar_width)
                            .clamp(0., range)
                    } else {
                        0.
                    }
                }
                Orientation::Vertical => {
                    let range = (content_size.y - visible_size.y).max(0.);
                    let scrollbar_height = (node.size().y * node.inverse_scale_factor
                        - scrollbar.min_thumb_size)
                        .max(1.0);
                    scroll_pos.offset_y = if range > 0. {
                        (drag.offset + (distance.y * content_size.y) / scrollbar_height)
                            .clamp(0., range)
                    } else {
                        0.
                    }
                }
            };
        }
    }
}

fn scrollbar_on_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    mut q_scrollbar: Query<(&CoreScrollbar, &mut ScrollbarDragState)>,
) {
    if let Ok((_scrollbar, mut drag)) = q_scrollbar.get_mut(ev.target()) {
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

        for child in children {
            if let Ok(mut thumb) = q_thumb.get_mut(*child) {
                match scrollbar.orientation {
                    Orientation::Horizontal => {
                        let thumb_size = if content_size.x > visible_size.x {
                            (track_length.x * visible_size.x / content_size.x)
                                .max(scrollbar.min_thumb_size)
                                .min(track_length.x)
                        } else {
                            track_length.x
                        };

                        let thumb_pos = if content_size.x > visible_size.x {
                            scroll_area.0.offset_x * (track_length.x - thumb_size)
                                / (content_size.x - visible_size.x)
                        } else {
                            0.
                        };

                        thumb.top = Val::Px(0.);
                        thumb.bottom = Val::Px(0.);
                        thumb.left = Val::Px(thumb_pos);
                        thumb.width = Val::Px(thumb_size);
                    }
                    Orientation::Vertical => {
                        let thumb_size = if content_size.y > visible_size.y {
                            (track_length.y * visible_size.y / content_size.y)
                                .max(scrollbar.min_thumb_size)
                                .min(track_length.y)
                        } else {
                            track_length.y
                        };

                        let thumb_pos = if content_size.y > visible_size.y {
                            scroll_area.0.offset_y * (track_length.y - thumb_size)
                                / (content_size.y - visible_size.y)
                        } else {
                            0.
                        };

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
            .add_observer(scrollbar_on_drag)
            .add_systems(PostUpdate, update_scrollbar_thumb);
    }
}
