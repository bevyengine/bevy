use bevy_ecs::{
    entity::Entity,
    event::{EventReader, EventWriter},
    prelude::{Added, With},
    system::{Commands, NonSendMut, Query, RemovedComponents}, schedule::IntoRunCriteria,
};
use bevy_math::IVec2;
use bevy_utils::tracing::error;
use bevy_window::{
    CloseWindowCommand, CreateWindow, SetCursorIconCommand, SetCursorLockModeCommand,
    SetCursorPositionCommand, SetCursorVisibilityCommand, SetDecorationsCommand,
    SetMaximizedCommand, SetMinimizedCommand, SetPositionCommand, SetPresentModeCommand,
    SetResizableCommand, SetResizeConstraintsCommand, SetResolutionCommand, SetScaleFactorCommand,
    SetTitleCommand, SetWindowModeCommand, Window, WindowBundle, WindowClosed, WindowCreated,
    WindowCursor, WindowCursorPosition, WindowDecorated, WindowMaximized, WindowMinimized,
    WindowModeComponent, WindowPosition, WindowPresentation, WindowResizable, WindowResolution,
    WindowScaleFactorChanged, WindowTitle, WindowTransparent, CursorIcon, WindowCurrentlyFocused, PresentMode, WindowHandle, RawWindowHandleWrapper, WindowCanvas, WindowResizeConstraints,
};
use raw_window_handle::HasRawWindowHandle;
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event_loop::EventLoop,
};

use crate::{converters, get_best_videomode, get_fitting_videomode, WinitWindows};

// TODO: Docs
/// System responsible for creating new windows whenever the Event<CreateWindow> has been sent
pub(crate) fn create_windows(
    mut commands: Commands,
    event_loop: NonSendMut<EventLoop<()>>, //  &EventLoopWindowTarget<()>, // TODO: Not sure how this would work
    mut create_window_events: EventReader<CreateWindow>,
    mut window_created_events: EventWriter<WindowCreated>,
    mut winit_windows: NonSendMut<WinitWindows>,
) {
    for event in create_window_events.iter() {
        // TODO: This should be about spawning the WinitWindow that corresponds
        let winit_window =
            winit_windows.create_window(&event_loop, event.entity, &event.descriptor);

        let mut entity_commands = commands.entity(event.entity);

        // Prepare data
        let position = winit_window
        .outer_position()
        .ok()
        .map(|position| IVec2::new(position.x, position.y));
        let inner_size = winit_window.inner_size();

        entity_commands.insert_bundle(WindowBundle {
            window: Window,
            handle: WindowHandle { raw_window_handle: RawWindowHandleWrapper::new(winit_window.raw_window_handle()) },
            presentation: WindowPresentation { present_mode: event.descriptor.present_mode },
            mode: WindowModeComponent { mode: event.descriptor.mode },
            position: WindowPosition { position },
            resolution: WindowResolution {
                requested_width: event.descriptor.width,
                requested_height: event.descriptor.height,
                physical_width: inner_size.width,
                physical_height: inner_size.height,
                scale_factor_override: event.descriptor.scale_factor_override,
                backend_scale_factor: winit_window.scale_factor(),
            },
            title: WindowTitle {
                title: event.descriptor.title.clone(),
            },
            cursor_position: WindowCursorPosition { physical_cursor_position: None }, 
            cursor: WindowCursor {
                cursor_icon: CursorIcon::Default,
                cursor_visible: event.descriptor.cursor_visible,
                cursor_locked: event.descriptor.cursor_locked,
            },
            canvas: WindowCanvas {
                canvas: event.descriptor.canvas.clone(),
                fit_canvas_to_parent: event.descriptor.fit_canvas_to_parent
            },
            resize_constraints: event.descriptor.resize_constraints,
            // TODO: All new windows must be focused?
            focused: WindowCurrentlyFocused, 
        });

        // Optional marker components
        if event.descriptor.resizable {
            entity_commands.insert(WindowResizable);
        }

        if event.descriptor.decorations {
            entity_commands.insert(WindowDecorated);
        }

        if event.descriptor.transparent {
            entity_commands.insert(WindowTransparent);
        }

        // TODO: Replace with separete `window_added`-system? See below
        window_created_events.send(WindowCreated {
            entity: event.entity,
        });

        // TODO: Fix this
        #[cfg(target_arch = "wasm32")]
        {
            let channel = world.resource_mut::<web_resize::CanvasParentResizeEventChannel>();
            if create_window_event.descriptor.fit_canvas_to_parent {
                let selector = if let Some(selector) = &create_window_event.descriptor.canvas {
                    selector
                } else {
                    web_resize::WINIT_CANVAS_SELECTOR
                };
                channel.listen_to_selector(create_window_event.entity, selector);
            }
        }
    }
}

// TODO: Docs
/// System that sends a [`WindowCreated`] event once a new [`Window`] component has been created
pub(crate) fn window_added(
    q: Query<Entity, Added<Window>>,
    mut writer: EventWriter<WindowCreated>,
) {
    for entity in q.iter() {
        writer.send(WindowCreated { entity });
    }
}

// TODO: Docs
/// System responsible for destroying windows from commands
pub(crate) fn destroy_windows(
    mut commands: Commands,
    mut close_window_writer: EventWriter<WindowClosed>,
    mut winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<CloseWindowCommand>,
) {
    for event in command_reader.iter() {
        // Close the OS window. (The `Drop` impl actually closes the window)
        let _ = winit_windows.remove_window(event.entity);

        // Despawn the entity from the world
        commands.entity(event.entity).despawn();

        // Send event that the window has been closed
        // TODO: Consider using the system below instead
        close_window_writer.send(WindowClosed {
            entity: event.entity,
        });
    }
}

// TODO: Docs
// TODO: Not sure if this is correct / better
/// System that detect that a window has been destroyed and sends an event as a result
pub(crate) fn window_destroyed(
    removed: RemovedComponents<Window>,
    mut writer: EventWriter<WindowClosed>,
) {
    for entity in removed.iter() {
        writer.send(WindowClosed { entity });
    }
}

// TODO: Docs
pub(crate) fn update_title(
    mut titles: Query<&mut WindowTitle, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut reader: EventReader<SetTitleCommand>,
) {
    for event in reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();
        // Set the winit title
        winit_window.set_title(&event.title);
        // Set the title in the component
        if let Ok(mut window_title) = titles.get_mut(event.entity) {
            // TODO: Remove the clone and somehow appease the borrow-checker instead
            window_title.update_title_from_backend(event.title.clone());
        } else {
            panic!("No WindowTitle on the entity in question");
        }
    }
}

// TODO: Docs
pub(crate) fn update_window_mode(
    mut window_modes: Query<&mut WindowModeComponent, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetWindowModeCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        // Update Winit Window
        match event.mode {
            bevy_window::WindowMode::BorderlessFullscreen => {
                winit_window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            }
            bevy_window::WindowMode::Fullscreen => {
                winit_window.set_fullscreen(Some(winit::window::Fullscreen::Exclusive(
                    get_best_videomode(&winit_window.current_monitor().unwrap()),
                )));
            }
            bevy_window::WindowMode::SizedFullscreen => {
                let (width, height) = event.resolution;
                winit_window.set_fullscreen(Some(winit::window::Fullscreen::Exclusive(
                    get_fitting_videomode(&winit_window.current_monitor().unwrap(), width, height),
                )))
            }
            bevy_window::WindowMode::Windowed => winit_window.set_fullscreen(None),
        }

        // Update components correspondinly
        // TODO: Should also update resolution?
        if let Ok(mut window_mode) = window_modes.get_mut(event.entity) {
            window_mode.update_mode_from_backend(event.mode);
        }
    }
}

// TODO: Docs
pub(crate) fn update_resolution(
    mut components: Query<&mut WindowResolution, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetResolutionCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        let (width, height) = event.logical_resolution;

        let physical_size =
            winit::dpi::LogicalSize::new(width, height).to_physical::<f64>(event.scale_factor);

        // Update Winit
        winit_window.set_inner_size(physical_size);

        // Update components
        if let Ok(mut window_resolution) = components.get_mut(event.entity) {
            // TODO: Is this casting f64 -> u32 correct / ok?
            window_resolution.update_actual_size_from_backend(
                physical_size.width as u32,
                physical_size.height as u32,
            );
        } else {
            // TODO: helpful panic comment
            panic!();
        }
    }
}

// TODO: Docs
pub(crate) fn update_cursor_position(
    mut components: Query<&mut WindowCursorPosition, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetCursorPositionCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        // Update Winit
        // TODO: Fix type inconsitencies
        let inner_size = winit_window
            .inner_size()
            .to_logical::<f64>(winit_window.scale_factor());

        // This can take either a physical position (physical pixels <i32>)
        // or logical position (logical pixels<f64>)
        winit_window
            .set_cursor_position(winit::dpi::LogicalPosition::new(
                event.position.x,
                inner_size.height - event.position.y,
            ))
            .unwrap_or_else(|e| error!("Unable to set cursor position: {}", e));

        // Update components
        if let Ok(mut cursor_position) = components.get_mut(event.entity) {
            cursor_position.update_position_from_backend(Some(event.position));
        } else {
            // TODO: helpful panic comment
            panic!();
        }
    }
}

// TODO: Docs
// Does this need to be a command?
// TODO: Check where this is actually being used
pub(crate) fn update_resize_contraints(
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetResizeConstraintsCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        // Update Winit
        let constraints = event.resize_constraints.check_constraints();
        let min_inner_size = LogicalSize {
            width: constraints.min_width,
            height: constraints.min_height,
        };
        let max_inner_size = LogicalSize {
            width: constraints.max_width,
            height: constraints.max_height,
        };

        winit_window.set_min_inner_size(Some(min_inner_size));
        if constraints.max_width.is_finite() && constraints.max_height.is_finite() {
            winit_window.set_max_inner_size(Some(max_inner_size));
        }

        // Update components
    }
}

// TODO: Docs
pub(crate) fn update_cursor_icon(
    mut components: Query<&mut WindowCursor, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetCursorIconCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        // Update Winit
        winit_window.set_cursor_icon(converters::convert_cursor_icon(event.icon));

        // Update components
        if let Ok(mut window_cursor) = components.get_mut(event.entity) {
            window_cursor.set_icon_from_backend(event.icon);
        } else {
            // TODO: helpful panic comment
            panic!();
        }
    }
}

// TODO: Docs
pub(crate) fn update_cursor_lock_mode(
    mut components: Query<&mut WindowCursor, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetCursorLockModeCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        // Update Winit
        winit_window
            .set_cursor_grab(event.locked)
            .unwrap_or_else(|e| error!("Unable to un/grab cursor: {}", e));

        // Update components
        if let Ok(mut window_cursor) = components.get_mut(event.entity) {
            window_cursor.set_locked_from_backend(event.locked);
        } else {
            // TODO: helpful panic comment
            panic!();
        }
    }
}

// TODO: Docs
pub(crate) fn update_cursor_visibility(
    mut components: Query<&mut WindowCursor, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetCursorVisibilityCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        // Update Winit
        winit_window.set_cursor_visible(event.visible);

        // Update components
        if let Ok(mut window_cursor) = components.get_mut(event.entity) {
            window_cursor.set_visible_from_backend(event.visible);
        } else {
            // TODO: helpful panic comment
            panic!();
        }
    }
}

// TODO: Docs
pub(crate) fn update_present_mode(
    mut components: Query<&mut WindowPresentation, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetPresentModeCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        // Update Winit
        // Present mode is only relevant for the renderer, so no need to do anything to Winit at this point

        // Update components
        if let Ok(mut window_presentation) = components.get_mut(event.entity) {
            window_presentation.update_present_mode_from_backend(event.present_mode);
        } else {
            // TODO: helpful panic comment
            panic!();
        }
    }
}

// TODO: Docs
pub(crate) fn update_scale_factor(
    mut components: Query<&mut WindowResolution, With<Window>>,
    mut window_dpi_changed_events: EventWriter<WindowScaleFactorChanged>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetScaleFactorCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        window_dpi_changed_events.send(WindowScaleFactorChanged {
            entity: event.entity,
            scale_factor: event.scale_factor,
        });

        if let Ok(mut window_resolution) = components.get_mut(event.entity) {
            // TODO: Should this be scale_factor_override instead?
            window_resolution.update_scale_factor_from_backend(event.scale_factor);
        } else {
            // TODO: Helpful panic comment
            panic!();
        }
    }
}

// TODO: Docs
// TODO: What happens if you try to apply decorations on something that already has decorations, or vice versa?
pub(crate) fn update_decorations(
    mut commands: Commands,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetDecorationsCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();
        winit_window.set_decorations(event.decorations);

        if event.decorations {
            // Add decoratiosn marker
            commands.entity(event.entity).insert(WindowDecorated);
        } else {
            // remove decoration marker
            commands.entity(event.entity).remove::<WindowDecorated>();
        }
    }
}

// TODO: Docs
// TODO: What happens if you try to apply resizable on something that already is resizable, or vice versa?
pub(crate) fn update_resizable(
    mut commands: Commands,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetResizableCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        winit_window.set_resizable(event.resizable);

        if event.resizable {
            // Add marker
            commands.entity(event.entity).insert(WindowResizable);
        } else {
            // remove marker
            commands.entity(event.entity).remove::<WindowResizable>();
        }
    }
}

// TODO: Docs
pub(crate) fn update_position(
    mut components: Query<&mut WindowPosition, With<Window>>,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetPositionCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        winit_window.set_outer_position(PhysicalPosition {
            x: event.position[0],
            y: event.position[1],
        });

        // TODO: When will position be Option<> ?
        if let Ok(mut comp) = components.get_mut(event.entity) {
            comp.update_actual_position_from_backend(event.position);
        } else {
            // TODO: helpful panic comment
            panic!()
        }
    }
}

// TODO: Docs
// TODO: What happens if you try to apply minimize on something that already is minimized, or vice versa?
pub(crate) fn update_minimized(
    mut commands: Commands,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetMinimizedCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        winit_window.set_minimized(event.minimized);

        if event.minimized {
            // Add marker
            commands.entity(event.entity).insert(WindowMinimized);
        } else {
            // remove marker
            commands.entity(event.entity).remove::<WindowMinimized>();
        }
    }
}

// TODO: Docs
// TODO: What happens if you try to apply maximize on something that already is maximized, or vice versa?
pub(crate) fn update_maximized(
    mut commands: Commands,
    winit_windows: NonSendMut<WinitWindows>,
    mut command_reader: EventReader<SetMaximizedCommand>,
) {
    for event in command_reader.iter() {
        let winit_window = winit_windows.get_window(event.entity).unwrap();

        winit_window.set_maximized(event.maximized);

        if event.maximized {
            // Add marker
            commands.entity(event.entity).insert(WindowMaximized);
        } else {
            // remove marker
            commands.entity(event.entity).remove::<WindowMaximized>();
        }
    }
}
