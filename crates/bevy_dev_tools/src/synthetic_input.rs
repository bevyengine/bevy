//! Input written as a window backend writes it, for tools that drive an app from outside, such as
//! end-to-end tests.
//!
//! [`click`] queues a click in [`InputQueue`], as [`WindowEvent`]s one frame apart, and
//! [`write_due_input`] writes each when its frame comes. Bevy's remote protocol exposes this as
//! its `input.click` method.

use alloc::collections::VecDeque;
use core::fmt::{self, Display, Formatter};

use bevy_camera::{NormalizedRenderTarget, RenderTarget};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    entity::{ContainsEntity, Entity},
    hierarchy::ChildOf,
    message::MessageWriter,
    query::With,
    resource::Resource,
    system::{Query, Res, ResMut},
    world::World,
};
use bevy_input::{
    mouse::{MouseButton, MouseButtonInput},
    ButtonState,
};
use bevy_math::Vec2;
use bevy_ui::{ComputedUiTargetCamera, ScrollIntoView, ScrollPosition, UiGlobalTransform};
use bevy_window::{PrimaryWindow, RawCursorMoved, Window, WindowEvent};

/// Input waiting for [`write_due_input`], each event with the frame that reads it, in order.
#[derive(Resource, Default)]
pub struct InputQueue {
    events: VecDeque<(u32, QueuedInput)>,
    /// The frame that reads the last event queued.
    last: u32,
}

enum QueuedInput {
    /// The cursor moves to `node` as it's drawn by then, or to `position` without one or once the
    /// node has gone.
    Cursor {
        window: Entity,
        node: Option<Entity>,
        position: Vec2,
    },
    Button {
        window: Entity,
        button: MouseButton,
        state: ButtonState,
    },
}

impl InputQueue {
    /// Queues `event` to be read `gap` frames after the last event queued, and not before
    /// `earliest`. Returns the frame that reads it.
    fn push(&mut self, earliest: u32, gap: u32, event: QueuedInput) -> u32 {
        let frame = earliest.max(self.last + gap);
        self.events.push_back((frame, event));
        self.last = frame;
        frame
    }
}

/// What [`click`] clicks on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClickTarget {
    /// The center of a UI node, in the window its camera renders to.
    ///
    /// A node hidden by a scrolled ancestor is scrolled into view first, with [`ScrollIntoView`].
    Node(Entity),
    /// A point in a window, in physical pixels.
    Point {
        /// The window to click in.
        window: Entity,
        /// The point to click, in physical pixels from the window's top left.
        position: Vec2,
    },
}

/// Why [`click`] couldn't queue a click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickError {
    /// The click was to be held for zero frames.
    ZeroHold,
    /// The target entity isn't a UI node.
    NotANode(Entity),
    /// The target node is drawn to something other than a window, such as an image.
    NotDrawnToWindow(Entity),
    /// The target node's camera has no window, and there's no primary window either.
    NoWindow(Entity),
    /// The target window isn't a window.
    NotAWindow(Entity),
}

impl Display for ClickError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroHold => write!(f, "a click is held for at least one frame"),
            Self::NotANode(entity) => write!(f, "{entity} is not a UI node"),
            Self::NotDrawnToWindow(entity) => write!(f, "{entity} is not drawn to a window"),
            Self::NoWindow(entity) => write!(f, "{entity} has no window to be drawn in"),
            Self::NotAWindow(entity) => write!(f, "{entity} is not a window"),
        }
    }
}

impl core::error::Error for ClickError {}

/// Queues a click on `target` with `button`, held for `hold` frames.
///
/// The click is three [`WindowEvent`]s, each read on its own frame: the cursor moves to the target,
/// the button is pressed, and the button is released `hold` frames after the press. Picking needs
/// the cursor to move before the press, and an app may treat a press and release on the same frame
/// differently from a click. A click queued while another is pending follows it.
///
/// Returns the frame that reads the release, as counted by [`FrameCount`]. This assumes
/// [`write_due_input`] runs after [`Last`](bevy_app::Last), where [`FrameCount`] advances, so
/// events it writes are read on the frame [`FrameCount`] names.
pub fn click(
    world: &mut World,
    target: ClickTarget,
    button: MouseButton,
    hold: u32,
) -> Result<u32, ClickError> {
    if hold == 0 {
        return Err(ClickError::ZeroHold);
    }
    let mut earliest = world
        .get_resource::<FrameCount>()
        .map_or(0, |frame| frame.0);
    let (window, node, position) = match target {
        ClickTarget::Node(entity) => {
            let (window, center) = locate(world, entity)?;
            if scroll_into_view(world, entity) {
                // Layout moves the node on the next frame; the cursor follows it there.
                earliest += 1;
            }
            (window, Some(entity), center)
        }
        ClickTarget::Point { window, position } => {
            if world.get::<Window>(window).is_none() {
                return Err(ClickError::NotAWindow(window));
            }
            (window, None, position)
        }
    };
    let mut queue = world.get_resource_or_init::<InputQueue>();
    queue.push(
        earliest,
        1,
        QueuedInput::Cursor {
            window,
            node,
            position,
        },
    );
    queue.push(
        earliest,
        1,
        QueuedInput::Button {
            window,
            button,
            state: ButtonState::Pressed,
        },
    );
    Ok(queue.push(
        earliest,
        hold,
        QueuedInput::Button {
            window,
            button,
            state: ButtonState::Released,
        },
    ))
}

/// The window `entity` is drawn in, and its center in physical pixels.
fn locate(world: &mut World, entity: Entity) -> Result<(Entity, Vec2), ClickError> {
    let (Some(transform), Some(camera)) = (
        world.get::<UiGlobalTransform>(entity),
        world.get::<ComputedUiTargetCamera>(entity),
    ) else {
        return Err(ClickError::NotANode(entity));
    };
    let center = transform.translation;
    let camera = camera.get();
    let primary = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .ok();
    let window = match camera {
        Some(camera) => match world
            .get::<RenderTarget>(camera)
            .and_then(|target| target.normalize(primary))
        {
            Some(NormalizedRenderTarget::Window(window)) => Some(window.entity()),
            _ => return Err(ClickError::NotDrawnToWindow(entity)),
        },
        None => primary,
    };
    match window {
        Some(window) if world.get::<Window>(window).is_some() => Ok((window, center)),
        _ => Err(ClickError::NoWindow(entity)),
    }
}

/// Triggers [`ScrollIntoView`] on `entity`, and returns whether any ancestor scrolled.
fn scroll_into_view(world: &mut World, entity: Entity) -> bool {
    let positions = |world: &World| {
        core::iter::successors(world.get::<ChildOf>(entity), |child_of| {
            world.get::<ChildOf>(child_of.parent())
        })
        .filter_map(|child_of| world.get::<ScrollPosition>(child_of.parent()))
        .map(|position| position.0)
        .collect::<Vec<_>>()
    };
    let before = positions(world);
    world.trigger(ScrollIntoView::new(entity));
    positions(world) != before
}

/// Writes the [`InputQueue`]'s events that are due, as [`WindowEvent`]s.
///
/// Run it after [`Last`](bevy_app::Last), so what it writes is read on the frame [`FrameCount`]
/// names: picking reads the cursor in `First`, and `bevy_window` splits the rest into typed
/// messages in `PreUpdate`. A cursor move to a node goes to the node's center as it is drawn when
/// the event is written.
pub fn write_due_input(
    mut queue: ResMut<InputQueue>,
    frame: Res<FrameCount>,
    nodes: Query<&UiGlobalTransform>,
    mut window_events: MessageWriter<WindowEvent>,
) {
    while queue.events.front().is_some_and(|(due, _)| *due <= frame.0) {
        let Some((_, event)) = queue.events.pop_front() else {
            break;
        };
        window_events.write(match event {
            QueuedInput::Cursor {
                window,
                node,
                position,
            } => {
                let position = node
                    .and_then(|node| nodes.get(node).ok())
                    .map_or(position, |transform| transform.translation);
                WindowEvent::CursorMoved(RawCursorMoved {
                    window,
                    physical_position: position.as_dvec2(),
                })
            }
            QueuedInput::Button {
                window,
                button,
                state,
            } => WindowEvent::MouseButtonInput(MouseButtonInput {
                button,
                state,
                window,
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{message::Messages, observer::On};

    /// A world at frame 10, with a primary window.
    fn world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<InputQueue>();
        world.init_resource::<Messages<WindowEvent>>();
        world.insert_resource(FrameCount(10));
        let window = world.spawn((Window::default(), PrimaryWindow)).id();
        (world, window)
    }

    fn node(world: &mut World, x: f32, y: f32) -> Entity {
        world
            .spawn((
                UiGlobalTransform::from_translation(Vec2::new(x, y)),
                ComputedUiTargetCamera::default(),
            ))
            .id()
    }

    /// Runs frames from the current one through `last`, and returns what each wrote, with the frame
    /// that reads it.
    fn frames_through(world: &mut World, last: u32) -> Vec<(u32, String)> {
        let mut written = Vec::new();
        loop {
            let frame = world.resource::<FrameCount>().0;
            world.run_system_cached(write_due_input).unwrap();
            let events = world
                .resource_mut::<Messages<WindowEvent>>()
                .drain()
                .collect::<Vec<_>>();
            written.extend(events.into_iter().map(|event| (frame, described(event))));
            if frame >= last {
                return written;
            }
            world.resource_mut::<FrameCount>().0 += 1;
        }
    }

    fn described(event: WindowEvent) -> String {
        match event {
            WindowEvent::CursorMoved(moved) => {
                let at = moved.physical_position;
                format!("move {} {}", at.x, at.y)
            }
            WindowEvent::MouseButtonInput(input) => format!("{:?} {:?}", input.state, input.button),
            other => format!("{other:?}"),
        }
    }

    fn events(list: &[(u32, &str)]) -> Vec<(u32, String)> {
        list.iter()
            .map(|&(frame, event)| (frame, event.to_string()))
            .collect()
    }

    #[test]
    fn a_click_is_three_events_on_three_frames() {
        let (mut world, _) = world();
        let start = node(&mut world, 400.0, 300.0);

        let frame = click(&mut world, ClickTarget::Node(start), MouseButton::Left, 1);

        assert_eq!(frame, Ok(12));
        assert_eq!(
            frames_through(&mut world, 13),
            events(&[
                (10, "move 400 300"),
                (11, "Pressed Left"),
                (12, "Released Left")
            ])
        );
    }

    #[test]
    fn hold_is_the_frames_from_press_to_release() {
        let (mut world, _) = world();
        let start = node(&mut world, 400.0, 300.0);

        let frame = click(&mut world, ClickTarget::Node(start), MouseButton::Left, 3);

        assert_eq!(frame, Ok(14));
        assert_eq!(
            frames_through(&mut world, 14),
            events(&[
                (10, "move 400 300"),
                (11, "Pressed Left"),
                (14, "Released Left")
            ])
        );
    }

    #[test]
    fn clicks_queue_one_after_another() {
        let (mut world, _) = world();
        let start = node(&mut world, 400.0, 300.0);
        let quit = node(&mut world, 400.0, 500.0);

        click(&mut world, ClickTarget::Node(start), MouseButton::Left, 1).unwrap();
        let frame = click(&mut world, ClickTarget::Node(quit), MouseButton::Left, 1);

        assert_eq!(frame, Ok(15));
        assert_eq!(
            frames_through(&mut world, 15)[3..],
            events(&[
                (13, "move 400 500"),
                (14, "Pressed Left"),
                (15, "Released Left")
            ])
        );
    }

    #[test]
    fn a_point_in_a_window_is_clicked_with_any_button() {
        let (mut world, window) = world();
        let target = ClickTarget::Point {
            window,
            position: Vec2::new(5.0, 6.0),
        };

        click(&mut world, target, MouseButton::Right, 1).unwrap();

        assert_eq!(
            frames_through(&mut world, 12),
            events(&[
                (10, "move 5 6"),
                (11, "Pressed Right"),
                (12, "Released Right")
            ])
        );
    }

    #[test]
    fn a_scrolled_node_is_clicked_where_layout_moves_it() {
        let (mut world, _) = world();
        let view = world.spawn(ScrollPosition::default()).id();
        let start = node(&mut world, 100.0, 250.0);
        world.entity_mut(start).insert(ChildOf(view));
        // Stands in for `bevy_ui`'s observer, which `UiPlugin` adds.
        world.add_observer(
            move |_: On<ScrollIntoView>, mut positions: Query<&mut ScrollPosition>| {
                positions.get_mut(view).unwrap().y = 160.0;
            },
        );

        let frame = click(&mut world, ClickTarget::Node(start), MouseButton::Left, 1);
        // What layout does on frame 10, having seen the scroll.
        frames_through(&mut world, 10);
        world
            .entity_mut(start)
            .insert(UiGlobalTransform::from_translation(Vec2::new(100.0, 90.0)));
        world.resource_mut::<FrameCount>().0 += 1;

        assert_eq!(frame, Ok(13));
        assert_eq!(
            frames_through(&mut world, 13),
            events(&[
                (11, "move 100 90"),
                (12, "Pressed Left"),
                (13, "Released Left")
            ])
        );
    }

    #[test]
    fn a_click_needs_a_target_and_a_hold() {
        let (mut world, window) = world();
        let start = node(&mut world, 400.0, 300.0);
        let other = world.spawn_empty().id();
        let point = |window| ClickTarget::Point {
            window,
            position: Vec2::ONE,
        };

        for (target, hold, error) in [
            (ClickTarget::Node(start), 0, ClickError::ZeroHold),
            (ClickTarget::Node(other), 1, ClickError::NotANode(other)),
            (point(other), 1, ClickError::NotAWindow(other)),
        ] {
            assert_eq!(
                click(&mut world, target, MouseButton::Left, hold),
                Err(error)
            );
        }
        world.despawn(window);
        assert_eq!(
            click(&mut world, point(window), MouseButton::Left, 1),
            Err(ClickError::NotAWindow(window))
        );
        assert_eq!(
            click(&mut world, ClickTarget::Node(start), MouseButton::Left, 1),
            Err(ClickError::NoWindow(start))
        );
        assert!(frames_through(&mut world, 12).is_empty());
    }
}
