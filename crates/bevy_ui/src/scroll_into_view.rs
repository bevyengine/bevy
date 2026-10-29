use bevy_ecs::{
    entity::Entity,
    event::EntityEvent,
    hierarchy::ChildOf,
    observer::On,
    query::{Or, With, Without},
    reflect::ReflectEvent,
    system::Query,
};
use bevy_math::{BVec2, Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use crate::{
    ComputedNode, FixedNode, GhostNode, IgnoreScroll, Node, OverflowAxis, ScrollPosition,
    UiGlobalTransform,
};

/// An event that scrolls the ancestors of a UI node so that the node is visible.
///
/// Each ancestor with [`OverflowAxis::Scroll`] on an axis has its [`ScrollPosition`] moved by the
/// least amount that shows the node on that axis, innermost ancestor first. If the node is bigger
/// than an ancestor's view, the view shows the edge the node is coming from.
/// [`container`](Self::container) can limit scrolling to the nearest scrolled ancestor.
///
/// This follows the DOM's [`Element.scrollIntoView()`], with `block` and `inline` both
/// `"nearest"`.
///
/// Positions are taken from the last layout, so a node spawned this frame is not scrolled into
/// view. Layout moves the node on the next frame.
///
/// [`Element.scrollIntoView()`]: https://drafts.csswg.org/cssom-view/#dom-element-scrollintoview
#[derive(EntityEvent, Reflect, Clone, Copy, Debug, PartialEq)]
#[reflect(Event, Clone, Debug, PartialEq)]
pub struct ScrollIntoView {
    /// The node to bring into view.
    pub entity: Entity,
    /// Which scrolled ancestors to scroll.
    pub container: ScrollIntoViewContainer,
}

impl ScrollIntoView {
    /// Scrolls every scrolled ancestor of `entity`.
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            container: ScrollIntoViewContainer::All,
        }
    }
}

/// Which ancestors a [`ScrollIntoView`] scrolls, after the DOM's [`ScrollIntoViewContainer`].
///
/// [`ScrollIntoViewContainer`]: https://drafts.csswg.org/cssom-view/#enumdef-scrollintoviewcontainer
#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[reflect(Clone, Debug, Default, PartialEq)]
pub enum ScrollIntoViewContainer {
    /// Every ancestor with [`OverflowAxis::Scroll`] on either axis.
    #[default]
    All,
    /// Only the nearest ancestor with [`OverflowAxis::Scroll`] on either axis.
    Nearest,
}

// Follows "scroll a target into view":
// https://drafts.csswg.org/cssom-view/#scroll-a-target-into-view
pub(crate) fn on_scroll_into_view(
    scroll: On<ScrollIntoView>,
    targets: Query<(&ComputedNode, &UiGlobalTransform)>,
    parents: Query<&ChildOf, Or<(Without<FixedNode>, With<GhostNode>)>>,
    ignore_scroll: Query<&IgnoreScroll>,
    mut views: Query<(
        &Node,
        &ComputedNode,
        &UiGlobalTransform,
        &mut ScrollPosition,
    )>,
) {
    let Ok((target_node, target_transform)) = targets.get(scroll.entity) else {
        return;
    };
    // In physical pixels, moved by each scroll as it is made, so an outer view sees where an
    // inner one put the target.
    let mut target = Rect::from_center_size(target_transform.translation, target_node.size());
    let mut child = scroll.entity;
    for ancestor in parents.iter_ancestors(scroll.entity) {
        let below = core::mem::replace(&mut child, ancestor);
        let Ok((node, view, view_transform, mut position)) = views.get_mut(ancestor) else {
            continue;
        };
        let scrolls = BVec2::new(
            node.overflow.x == OverflowAxis::Scroll,
            node.overflow.y == OverflowAxis::Scroll,
        );
        if !scrolls.any() {
            continue;
        }
        // Scrolling `ancestor` doesn't move a child that ignores its scroll.
        let axes = scrolls
            & !ignore_scroll
                .get(below)
                .map_or(BVec2::FALSE, |ignore| ignore.0);

        // The scrollport: the padding box, less the scrollbar gutters.
        let mut port = view.padding_box();
        port.max -= view.scrollbar_size;
        let port = Rect {
            min: port.min + view_transform.translation,
            max: port.max + view_transform.translation,
        };
        // The same range layout clamps `ScrollPosition` to (taffy's `scroll_width` and
        // `scroll_height`).
        let max = (view.content_size + view.scrollbar_size.min(view.size) - view.size
            + view.border.min_inset
            + view.border.max_inset)
            .max(Vec2::ZERO);

        let current = view.scroll_position;
        let mut moved = Vec2::ZERO;
        for i in 0..2 {
            if !axes.test(i) {
                continue;
            }
            let delta = nearest_delta(target.min[i], target.max[i], port.min[i], port.max[i]);
            let new = (current[i] + delta).clamp(0.0, max[i]);
            if new != current[i] {
                moved[i] = new - current[i];
                // `ScrollPosition` is in logical pixels.
                position.0[i] = new * view.inverse_scale_factor();
            }
        }
        // Raising the scroll position moves the content up and left by the same amount.
        target.min -= moved;
        target.max -= moved;

        if scroll.container == ScrollIntoViewContainer::Nearest {
            return;
        }
    }
}

/// How far to scroll so that `start..end` is shown in `view_start..view_end`, by the "nearest"
/// rules of "determine the scroll-into-view position":
/// <https://drafts.csswg.org/cssom-view/#determine-the-scroll-into-view-position>
fn nearest_delta(start: f32, end: f32, view_start: f32, view_end: f32) -> f32 {
    // The spec compares sizes strictly, which leaves a target exactly as big as the view with no
    // rule; it's treated as smaller.
    let fits = end - start <= view_end - view_start;
    let before = start < view_start;
    let after = end > view_end;
    if before && after {
        0.0
    } else if (before && fits) || (after && !fits) {
        start - view_start
    } else if before || after {
        end - view_end
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    use crate::Overflow;

    /// A node scrolled vertically, 100 px wide, showing `top..top + height` of the window over
    /// `content` px of content.
    fn view(
        world: &mut World,
        parent: Option<Entity>,
        top: f32,
        height: f32,
        content: f32,
    ) -> Entity {
        let mut view = world.spawn((
            Node {
                overflow: Overflow::scroll_y(),
                ..Default::default()
            },
            ComputedNode {
                size: Vec2::new(100.0, height),
                content_size: Vec2::new(100.0, content),
                inverse_scale_factor: 1.0,
                ..Default::default()
            },
            UiGlobalTransform::from_translation(Vec2::new(50.0, top + height / 2.0)),
            ScrollPosition::default(),
        ));
        if let Some(parent) = parent {
            view.insert(ChildOf(parent));
        }
        view.id()
    }

    /// A node 100 px wide at `top..bottom` of the window.
    fn item(world: &mut World, parent: Entity, top: f32, bottom: f32) -> Entity {
        world
            .spawn((
                ChildOf(parent),
                ComputedNode {
                    size: Vec2::new(100.0, bottom - top),
                    inverse_scale_factor: 1.0,
                    ..Default::default()
                },
                UiGlobalTransform::from_translation(Vec2::new(50.0, (top + bottom) / 2.0)),
            ))
            .id()
    }

    fn scrolled_to(world: &mut World, view: Entity, scroll_y: f32) {
        world
            .get_mut::<ComputedNode>(view)
            .unwrap()
            .scroll_position
            .y = scroll_y;
        world.get_mut::<ScrollPosition>(view).unwrap().y = scroll_y;
    }

    fn scroll(world: &mut World, event: ScrollIntoView) {
        world.add_observer(on_scroll_into_view);
        world.trigger(event);
    }

    fn scroll_y(world: &World, view: Entity) -> f32 {
        world.get::<ScrollPosition>(view).unwrap().y
    }

    #[test]
    fn a_node_below_the_view_meets_its_bottom_edge() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 400.0);
        let item = item(&mut world, view, 240.0, 260.0);

        scroll(&mut world, ScrollIntoView::new(item));

        assert_eq!(scroll_y(&world, view), 160.0);
    }

    #[test]
    fn a_node_above_the_view_meets_its_top_edge() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 400.0);
        scrolled_to(&mut world, view, 200.0);
        let item = item(&mut world, view, -50.0, -30.0);

        scroll(&mut world, ScrollIntoView::new(item));

        assert_eq!(scroll_y(&world, view), 150.0);
    }

    #[test]
    fn a_visible_node_is_not_scrolled() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 400.0);
        let item = item(&mut world, view, 40.0, 60.0);

        scroll(&mut world, ScrollIntoView::new(item));

        assert_eq!(scroll_y(&world, view), 0.0);
    }

    #[test]
    fn a_node_bigger_than_the_view_shows_the_edge_it_comes_from() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 1000.0);
        let below = item(&mut world, view, 150.0, 300.0);

        scroll(&mut world, ScrollIntoView::new(below));
        assert_eq!(scroll_y(&world, view), 150.0);

        let mut world = World::new();
        let view = self::view(&mut world, None, 0.0, 100.0, 1000.0);
        scrolled_to(&mut world, view, 300.0);
        let above = item(&mut world, view, -200.0, -50.0);

        scroll(&mut world, ScrollIntoView::new(above));
        assert_eq!(scroll_y(&world, view), 150.0);
    }

    #[test]
    fn scrolling_stops_at_the_end_of_the_content() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 400.0);
        let item = item(&mut world, view, 450.0, 470.0);

        scroll(&mut world, ScrollIntoView::new(item));

        assert_eq!(scroll_y(&world, view), 300.0);
    }

    #[test]
    fn scroll_position_is_in_logical_pixels() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 400.0);
        world
            .get_mut::<ComputedNode>(view)
            .unwrap()
            .inverse_scale_factor = 0.5;
        let item = item(&mut world, view, 240.0, 260.0);

        scroll(&mut world, ScrollIntoView::new(item));

        assert_eq!(scroll_y(&world, view), 80.0);
    }

    // An outer view 100 px tall, over an inner view at 300..400 that holds an item at 640..660.
    fn nested(world: &mut World) -> (Entity, Entity, Entity) {
        let outer = view(world, None, 0.0, 100.0, 1000.0);
        let inner = view(world, Some(outer), 300.0, 100.0, 400.0);
        let item = item(world, inner, 640.0, 660.0);
        (outer, inner, item)
    }

    #[test]
    fn nested_views_are_scrolled_in_one_pass() {
        let mut world = World::new();
        let (outer, inner, item) = nested(&mut world);

        scroll(&mut world, ScrollIntoView::new(item));

        // The inner view brings the item to 380..400; the outer view then brings that to 80..100.
        assert_eq!(scroll_y(&world, inner), 260.0);
        assert_eq!(scroll_y(&world, outer), 300.0);
    }

    #[test]
    fn nearest_scrolls_only_the_nearest_view() {
        let mut world = World::new();
        let (outer, inner, item) = nested(&mut world);

        scroll(
            &mut world,
            ScrollIntoView {
                entity: item,
                container: ScrollIntoViewContainer::Nearest,
            },
        );

        assert_eq!(scroll_y(&world, inner), 260.0);
        assert_eq!(scroll_y(&world, outer), 0.0);
    }

    #[test]
    fn a_child_that_ignores_scroll_is_not_scrolled_to() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 400.0);
        let item = item(&mut world, view, 240.0, 260.0);
        world
            .entity_mut(item)
            .insert(IgnoreScroll(BVec2::new(false, true)));

        scroll(&mut world, ScrollIntoView::new(item));

        assert_eq!(scroll_y(&world, view), 0.0);
    }

    #[test]
    fn a_fixed_node_does_not_scroll_its_ancestors() {
        let mut world = World::new();
        let view = view(&mut world, None, 0.0, 100.0, 400.0);
        let fixed = world.spawn((ChildOf(view), FixedNode)).id();
        let item = item(&mut world, fixed, 240.0, 260.0);

        scroll(&mut world, ScrollIntoView::new(item));

        assert_eq!(scroll_y(&world, view), 0.0);
    }
}
