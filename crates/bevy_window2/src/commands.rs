use bevy_ecs::{
    entity::Entity,
    event::Events,
    prelude::World,
    system::{Command, Commands},
};
use bevy_math::{IVec2, Vec2};

use crate::{CursorIcon, WindowMode, WindowPresentMode, WindowResizeConstraints};

/// Window commands sent to window backends
#[derive(Debug)]
pub enum WindowCommand {
    /// Set window mode and resolution
    SetWindowMode(WindowMode, (u32, u32)),
    /// Set window title
    SetTitle(String),
    /// Set window scale factor
    SetScaleFactor(f64),
    /// Set window resolution and scale factor
    SetResolution((f32, f32), f64),
    /// Set window present mode
    SetPresentMode(WindowPresentMode),
    /// Set whether window is resizeable
    SetResizable(bool),
    /// Set whether window has decorations
    SetDecorations(bool),
    /// Set cursor icon
    SetCursorIcon(CursorIcon),
    /// Set whether the cursor will be locked
    SetCursorLockMode(bool),
    /// Set whether the cursor will be visible
    SetCursorVisibility(bool),
    /// Set cursor position
    SetCursorPosition(Vec2),
    /// Sets the window to maximized or back
    SetMaximized(bool),
    /// Sets the window to minimized or back
    SetMinimized(bool),
    /// Set window position
    SetPosition(IVec2),
    /// Set window resize constraints
    SetResizeConstraints(WindowResizeConstraints),
}

/// An event that is sent when window commands have been queued
#[derive(Debug)]
pub struct WindowCommandQueued {
    /// Window id
    pub window_id: Entity,
    /// Queued command
    pub command: WindowCommand,
}

impl Command for WindowCommandQueued {
    fn write(self, world: &mut World) {
        let mut events = world
            .get_resource_mut::<Events<WindowCommandQueued>>()
            .unwrap();
        events.send(self);
    }
}

/// A list of commands that will be run to modify a window.
pub struct WindowCommands<'w, 's, 'a> {
    window_id: Entity,
    commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> WindowCommands<'w, 's, 'a> {
    /// Adds a window command directly to the command list.
    #[inline]
    pub fn add(&mut self, command: WindowCommand) -> &mut Self {
        self.commands.add(WindowCommandQueued {
            window_id: self.window_id,
            command,
        });
        self
    }

    /// Set window display mode
    #[inline]
    pub fn set_window_mode(&mut self, mode: WindowMode, resolution: (u32, u32)) -> &mut Self {
        self.add(WindowCommand::SetWindowMode(mode, resolution))
    }

    /// Set window title
    #[inline]
    pub fn set_title(&mut self, title: String) -> &mut Self {
        self.add(WindowCommand::SetTitle(title))
    }

    /// Set window scale factor
    #[inline]
    pub fn set_scale_factor(&mut self, scale_factor: f64) -> &mut Self {
        self.add(WindowCommand::SetScaleFactor(scale_factor))
    }

    /// Set window resolution
    #[inline]
    pub fn set_resolution(
        &mut self,
        logical_resolution: (f32, f32),
        scale_factor: f64,
    ) -> &mut Self {
        self.add(WindowCommand::SetResolution(
            logical_resolution,
            scale_factor,
        ))
    }

    /// Set window present mode
    #[inline]
    #[doc(alias = "set_vsync")]
    pub fn set_present_mode(&mut self, present_mode: WindowPresentMode) -> &mut Self {
        self.add(WindowCommand::SetPresentMode(present_mode))
    }

    /// Set whether the window can resize
    #[inline]
    pub fn set_resizable(&mut self, resizable: bool) -> &mut Self {
        self.add(WindowCommand::SetResizable(resizable))
    }

    /// Set whether the window should have decorations, e.g. borders, title bar
    #[inline]
    pub fn set_decorations(&mut self, decorations: bool) -> &mut Self {
        self.add(WindowCommand::SetDecorations(decorations))
    }

    /// Set window icon
    #[inline]
    pub fn set_cursor_icon(&mut self, icon: CursorIcon) -> &mut Self {
        self.add(WindowCommand::SetCursorIcon(icon))
    }

    /// Set whether the cursor will be locked
    #[inline]
    pub fn set_cursor_lock_mode(&mut self, locked: bool) -> &mut Self {
        self.add(WindowCommand::SetCursorLockMode(locked))
    }

    /// Set whether the cursor will be visible
    #[inline]
    pub fn set_cursor_visibility(&mut self, visible: bool) -> &mut Self {
        self.add(WindowCommand::SetCursorVisibility(visible))
    }

    /// Set cursor position
    #[inline]
    pub fn set_cursor_position(&mut self, position: Vec2) -> &mut Self {
        self.add(WindowCommand::SetCursorPosition(position))
    }

    /// Sets the window to maximized or back
    #[inline]
    pub fn set_maximized(&mut self, maximized: bool) -> &mut Self {
        self.add(WindowCommand::SetMaximized(maximized))
    }

    /// Sets the window to minimized or back
    ///
    /// # Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - Wayland: Un-minimize is unsupported.
    #[inline]
    pub fn set_minimized(&mut self, minimized: bool) -> &mut Self {
        self.add(WindowCommand::SetMinimized(minimized))
    }

    /// Modifies the position of the window in physical pixels.
    ///
    /// Note that the top-left hand corner of the desktop is not necessarily the same as the screen.
    /// If the user uses a desktop with multiple monitors, the top-left hand corner of the
    /// desktop is the top-left hand corner of the monitor at the top-left of the desktop. This
    /// automatically un-maximizes the window if it's maximized.
    ///
    /// # Platform-specific
    ///
    /// - iOS: Can only be called on the main thread. Sets the top left coordinates of the window in
    ///   the screen space coordinate system.
    /// - Web: Sets the top-left coordinates relative to the viewport.
    /// - Android / Wayland: Unsupported.
    #[inline]
    pub fn set_position(&mut self, position: IVec2) -> &mut Self {
        self.add(WindowCommand::SetPosition(position))
    }

    /// Set window resize constraints
    ///
    /// Modifies the minimum and maximum window bounds for resizing in logical pixels.
    #[inline]
    pub fn set_resize_constraints(
        &mut self,
        resize_constraints: WindowResizeConstraints,
    ) -> &mut Self {
        self.add(WindowCommand::SetResizeConstraints(resize_constraints))
    }
}

/// Extension trait for adding a [`WindowCommands`] helper for [`Commands`]
pub trait CommandsExt<'w, 's> {
    /// Returns an [`WindowCommands`] builder for the requested window.
    fn window<'a>(&'a mut self, window: Entity) -> WindowCommands<'w, 's, 'a>;
}

impl<'w, 's> CommandsExt<'w, 's> for Commands<'w, 's> {
    #[track_caller]
    fn window<'a>(&'a mut self, window_id: Entity) -> WindowCommands<'w, 's, 'a> {
        // currently there is no way of checking if an entity exists from `Commands`
        // some function like `exists` or `contains` should get added to `Commands`
        WindowCommands {
            window_id,
            commands: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        event::{EventReader, Events},
        prelude::World,
        schedule::{Stage, SystemStage},
        system::Commands,
    };
    use bevy_math::IVec2;

    use crate::{CommandsExt, WindowCommandQueued};

    fn create_commands(mut commands: Commands) {
        let first = commands.spawn().id();
        let mut first_cmds = commands.window(first);
        first_cmds
            .set_cursor_lock_mode(false)
            .set_title("first".to_string())
            .set_position(IVec2::ONE);

        let second = commands.spawn().id();
        let mut second_cmds = commands.window(second);
        second_cmds
            .set_cursor_lock_mode(true)
            .set_title("second".to_string())
            .set_position(IVec2::ZERO);
    }

    fn receive_commands(mut events: EventReader<WindowCommandQueued>) {
        let received = events
            .iter()
            .map(|WindowCommandQueued { window_id, command }| format!("{window_id:?}: {command:?}"))
            .collect::<Vec<_>>();
        let expected = vec![
            "0v0: SetCursorLockMode(false)".to_string(),
            "0v0: SetTitle(\"first\")".to_string(),
            "0v0: SetPosition(IVec2(1, 1))".to_string(),
            "1v0: SetCursorLockMode(true)".to_string(),
            "1v0: SetTitle(\"second\")".to_string(),
            "1v0: SetPosition(IVec2(0, 0))".to_string(),
        ];
        assert_eq!(received, expected);
    }

    #[test]
    fn test_window_commands() {
        let mut world = World::new();
        world.init_resource::<Events<WindowCommandQueued>>();

        SystemStage::single(create_commands).run(&mut world);
        SystemStage::single(receive_commands).run(&mut world);
    }
}
