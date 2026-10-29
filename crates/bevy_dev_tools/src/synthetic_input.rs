//! Mocked input, simulated as if it came from a windowing backend, designed for tools that drive an app from outside, such as
//! end-to-end tests.
//!
//! [`click`], [`key`] and [`type_text`] queue input in [`InputQueue`], as [`WindowEvent`]s one
//! frame apart, and [`write_due_input`] writes each when its frame comes. Bevy's remote protocol
//! exposes these as its `input.click`, `input.key` and `input.type` methods.

mod us_layout;

use alloc::{
    collections::VecDeque,
    string::{String, ToString},
};
use core::fmt::{self, Display, Formatter};

use bevy_camera::{Camera, NormalizedRenderTarget, RenderTarget};
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
    keyboard::{Key, KeyCode, KeyboardInput, NativeKeyCode},
    mouse::{MouseButton, MouseButtonInput},
    ButtonState,
};
use bevy_math::Vec2;
use bevy_ui::{ComputedUiTargetCamera, ScrollIntoView, ScrollPosition, UiGlobalTransform};
use bevy_window::{PrimaryWindow, RawCursorMoved, Window, WindowEvent};

/// Input waiting for [`write_due_input`], each event with the frame on which the app receives it, in order.
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
    Key {
        window: Entity,
        key_code: KeyCode,
        logical_key: Key,
        text: Option<String>,
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
/// Returns the frame on which the app receives the release, as counted by [`FrameCount`]. This assumes
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

/// What [`key`] does with a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    /// Presses the key, and releases it `hold` frames later.
    Tap {
        /// How many frames the key stays pressed.
        hold: u32,
    },
    /// Presses the key and leaves it pressed, such as a modifier held across other input.
    Press,
    /// Releases a pressed key.
    Release,
}

/// Why [`key`] or [`type_text`] couldn't queue input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyError {
    /// The key was to be held for zero frames.
    ZeroHold,
    /// There was no text to type.
    EmptyText,
    /// No logical key was given, and the key code isn't one a US keyboard layout maps.
    UnknownKey(KeyCode),
    /// No window was given, and there's no primary window.
    NoPrimaryWindow,
    /// The target window isn't a window.
    NotAWindow(Entity),
}

impl Display for KeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroHold => write!(f, "a key is held for at least one frame"),
            Self::EmptyText => write!(f, "there is no text to type"),
            Self::UnknownKey(key_code) => write!(
                f,
                "{key_code:?} has no logical key on a US keyboard; give one explicitly"
            ),
            Self::NoPrimaryWindow => write!(f, "there is no primary window"),
            Self::NotAWindow(entity) => write!(f, "{entity} is not a window"),
        }
    }
}

impl core::error::Error for KeyError {}

/// Queues a key press, release, or both, in `window` or the primary window.
///
/// `logical_key` and `text` are what the key means in the user's keyboard layout. When
/// `logical_key` is `None`, both come from a US layout, and `text` only overrides the text. Each
/// event is read on its own frame, after any input already queued.
///
/// Returns the frame on which the app receives the last event, as [`click`] does.
pub fn key(
    world: &mut World,
    window: Option<Entity>,
    key_code: KeyCode,
    logical_key: Option<Key>,
    text: Option<&str>,
    action: KeyAction,
) -> Result<u32, KeyError> {
    if action == (KeyAction::Tap { hold: 0 }) {
        return Err(KeyError::ZeroHold);
    }
    let window = keyboard_window(world, window)?;
    let (logical_key, text) = match logical_key {
        Some(logical_key) => (logical_key, text.map(ToString::to_string)),
        None => {
            let (logical_key, plain) =
                us_layout::logical_key(key_code).ok_or(KeyError::UnknownKey(key_code))?;
            let text = text
                .map(ToString::to_string)
                .or(plain.map(|plain| plain.to_string()));
            (logical_key, text)
        }
    };
    let event = |state| QueuedInput::Key {
        window,
        key_code,
        logical_key: logical_key.clone(),
        text: text.clone(),
        state,
    };
    let earliest = current_frame(world);
    let mut queue = world.get_resource_or_init::<InputQueue>();
    Ok(match action {
        KeyAction::Tap { hold } => {
            queue.push(earliest, 1, event(ButtonState::Pressed));
            queue.push(earliest, hold, event(ButtonState::Released))
        }
        KeyAction::Press => queue.push(earliest, 1, event(ButtonState::Pressed)),
        KeyAction::Release => queue.push(earliest, 1, event(ButtonState::Released)),
    })
}

/// Queues the key presses that type `text` on a US keyboard, in `window` or the primary window.
///
/// Each character is a press and a release of its key, with Shift pressed around it when the
/// character needs it, each event on its own frame. A character a US keyboard can't type is sent
/// with an unidentified key code, as its own logical key and text, so it still reaches a text
/// field.
///
/// Returns the frame on which the app receives the last event, as [`click`] does.
pub fn type_text(world: &mut World, window: Option<Entity>, text: &str) -> Result<u32, KeyError> {
    if text.is_empty() {
        return Err(KeyError::EmptyText);
    }
    let window = keyboard_window(world, window)?;
    let earliest = current_frame(world);
    let mut queue = world.get_resource_or_init::<InputQueue>();
    let mut frame = earliest;
    for character in text.chars() {
        let (key_code, shifted) = us_layout::character_key(character)
            .unwrap_or((KeyCode::Unidentified(NativeKeyCode::Unidentified), false));
        let typed = character.to_string();
        let logical_key = if character == ' ' {
            Key::Space
        } else {
            Key::Character(typed.as_str().into())
        };
        let shift = |state| QueuedInput::Key {
            window,
            key_code: KeyCode::ShiftLeft,
            logical_key: Key::Shift,
            text: None,
            state,
        };
        let event = |state| QueuedInput::Key {
            window,
            key_code,
            logical_key: logical_key.clone(),
            text: Some(typed.clone()),
            state,
        };
        if shifted {
            queue.push(earliest, 1, shift(ButtonState::Pressed));
        }
        queue.push(earliest, 1, event(ButtonState::Pressed));
        frame = queue.push(earliest, 1, event(ButtonState::Released));
        if shifted {
            frame = queue.push(earliest, 1, shift(ButtonState::Released));
        }
    }
    Ok(frame)
}

/// `window`, or the primary window.
fn keyboard_window(world: &mut World, window: Option<Entity>) -> Result<Entity, KeyError> {
    match window {
        Some(window) if world.get::<Window>(window).is_some() => Ok(window),
        Some(window) => Err(KeyError::NotAWindow(window)),
        None => world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world)
            .map_err(|_| KeyError::NoPrimaryWindow),
    }
}

fn current_frame(world: &World) -> u32 {
    world
        .get_resource::<FrameCount>()
        .map_or(0, |frame| frame.0)
}

/// The window `entity` is drawn in, and its center in physical pixels.
fn locate(world: &mut World, entity: Entity) -> Result<(Entity, Vec2), ClickError> {
    let (Some(transform), Some(camera)) = (
        world.get::<UiGlobalTransform>(entity),
        world.get::<ComputedUiTargetCamera>(entity),
    ) else {
        return Err(ClickError::NotANode(entity));
    };
    let camera = camera.get();
    let center = transform.translation
        + camera
            .and_then(|camera| world.get::<Camera>(camera))
            .and_then(Camera::physical_viewport_rect)
            .map_or(Vec2::ZERO, |viewport| viewport.min.as_vec2());
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
    nodes: Query<(&UiGlobalTransform, &ComputedUiTargetCamera)>,
    cameras: Query<&Camera>,
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
                let position = node.and_then(|node| nodes.get(node).ok()).map_or(
                    position,
                    |(transform, camera)| {
                        transform.translation
                            + camera
                                .get()
                                .and_then(|camera| cameras.get(camera).ok())
                                .and_then(Camera::physical_viewport_rect)
                                .map_or(Vec2::ZERO, |viewport| viewport.min.as_vec2())
                    },
                );
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
            QueuedInput::Key {
                window,
                key_code,
                logical_key,
                text,
                state,
            } => WindowEvent::KeyboardInput(KeyboardInput {
                key_code,
                logical_key,
                state,
                text: text.map(|text| text.as_str().into()),
                repeat: false,
                window,
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_camera::Viewport;
    use bevy_ecs::{message::Messages, observer::On};
    use bevy_math::UVec2;
    use bevy_reflect::structs::GetField;

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
            WindowEvent::KeyboardInput(input) => format!(
                "{:?} {:?} {:?} {:?}",
                input.state,
                input.key_code,
                input.logical_key,
                input.text.as_deref()
            ),
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
    fn a_node_in_an_offset_viewport_is_clicked_in_window_coordinates() {
        let (mut world, _) = world();
        let camera = world
            .spawn((
                Camera {
                    viewport: Some(Viewport {
                        physical_position: UVec2::new(200, 100),
                        physical_size: UVec2::new(400, 300),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                RenderTarget::default(),
            ))
            .id();
        let start = node(&mut world, 50.0, 40.0);
        // `bevy_ui` sets this while propagating target cameras, which needs its plugins.
        *world
            .get_mut::<ComputedUiTargetCamera>(start)
            .unwrap()
            .get_field_mut::<Option<Entity>>("camera")
            .unwrap() = Some(camera);

        click(&mut world, ClickTarget::Node(start), MouseButton::Left, 1).unwrap();
        // Moved before the cursor event is written, which follows it.
        world
            .entity_mut(start)
            .insert(UiGlobalTransform::from_translation(Vec2::new(60.0, 40.0)));

        assert_eq!(
            frames_through(&mut world, 10),
            events(&[(10, "move 260 140")])
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

    fn tap(world: &mut World, key_code: KeyCode, hold: u32) -> Result<u32, KeyError> {
        key(world, None, key_code, None, None, KeyAction::Tap { hold })
    }

    #[test]
    fn a_key_takes_its_meaning_from_a_us_layout() {
        let (mut world, _) = world();

        assert_eq!(tap(&mut world, KeyCode::KeyA, 1), Ok(11));
        assert_eq!(tap(&mut world, KeyCode::Escape, 3), Ok(15));

        assert_eq!(
            frames_through(&mut world, 15),
            events(&[
                (10, r#"Pressed KeyA Character("a") Some("a")"#),
                (11, r#"Released KeyA Character("a") Some("a")"#),
                (12, "Pressed Escape Escape None"),
                (15, "Released Escape Escape None"),
            ])
        );
    }

    #[test]
    fn a_key_can_be_given_its_meaning_and_pressed_and_released_apart() {
        let (mut world, window) = world();
        let mut numpad_plus = |action| {
            let plus = Key::Character("+".into());
            key(
                &mut world,
                Some(window),
                KeyCode::NumpadAdd,
                Some(plus),
                Some("+"),
                action,
            )
        };

        numpad_plus(KeyAction::Press).unwrap();
        let frame = numpad_plus(KeyAction::Release);

        assert_eq!(frame, Ok(11));
        assert_eq!(
            frames_through(&mut world, 11),
            events(&[
                (10, r#"Pressed NumpadAdd Character("+") Some("+")"#),
                (11, r#"Released NumpadAdd Character("+") Some("+")"#),
            ])
        );
    }

    #[test]
    fn typing_holds_shift_where_a_character_needs_it() {
        let (mut world, _) = world();

        let frame = type_text(&mut world, None, "a B");

        assert_eq!(frame, Ok(17));
        assert_eq!(
            frames_through(&mut world, 17),
            events(&[
                (10, r#"Pressed KeyA Character("a") Some("a")"#),
                (11, r#"Released KeyA Character("a") Some("a")"#),
                (12, r#"Pressed Space Space Some(" ")"#),
                (13, r#"Released Space Space Some(" ")"#),
                (14, "Pressed ShiftLeft Shift None"),
                (15, r#"Pressed KeyB Character("B") Some("B")"#),
                (16, r#"Released KeyB Character("B") Some("B")"#),
                (17, "Released ShiftLeft Shift None"),
            ])
        );
    }

    #[test]
    fn a_character_off_the_layout_is_still_typed() {
        let (mut world, _) = world();

        type_text(&mut world, None, "é").unwrap();

        assert_eq!(
            frames_through(&mut world, 10),
            events(&[(
                10,
                r#"Pressed Unidentified(Unidentified) Character("é") Some("é")"#
            )])
        );
    }

    #[test]
    fn key_input_needs_a_window_and_something_to_send() {
        let (mut world, window) = world();
        let other = world.spawn_empty().id();

        assert_eq!(tap(&mut world, KeyCode::KeyA, 0), Err(KeyError::ZeroHold));
        assert_eq!(
            tap(&mut world, KeyCode::NumpadAdd, 1),
            Err(KeyError::UnknownKey(KeyCode::NumpadAdd))
        );
        assert_eq!(type_text(&mut world, None, ""), Err(KeyError::EmptyText));
        assert_eq!(
            type_text(&mut world, Some(other), "a"),
            Err(KeyError::NotAWindow(other))
        );
        world.despawn(window);
        assert_eq!(
            type_text(&mut world, None, "a"),
            Err(KeyError::NoPrimaryWindow)
        );
        assert!(frames_through(&mut world, 12).is_empty());
    }
}
