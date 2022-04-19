use bevy_ecs::{
    entity::Entity,
    event::Events,
    prelude::World,
    system::{Command, Commands},
};
use bevy_math::{IVec2, Vec2};

use crate::{
    CursorIcon, PresentMode, WindowCommand, WindowCommandQueued, WindowMode,
    WindowResizeConstraints,
};

struct WindowInternalCommand {
    window: Entity,
    command: WindowCommand,
}

impl Command for WindowInternalCommand {
    fn write(self, world: &mut World) {
        let mut events = world
            .get_resource_mut::<Events<WindowCommandQueued>>()
            .unwrap();
        let WindowInternalCommand { window, command } = self;
        events.send(WindowCommandQueued { window, command });
    }
}

pub struct WindowCommands<'w, 's, 'a> {
    window: Entity,
    commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> WindowCommands<'w, 's, 'a> {
    #[inline]
    pub fn push(&mut self, command: WindowCommand) -> &mut Self {
        self.commands.add(WindowInternalCommand {
            window: self.window,
            command,
        });
        self
    }

    #[inline]
    pub fn set_window_mode(&mut self, mode: WindowMode, resolution: (u32, u32)) -> &mut Self {
        self.push(WindowCommand::SetWindowMode { mode, resolution })
    }

    #[inline]
    pub fn set_title(&mut self, title: String) -> &mut Self {
        self.push(WindowCommand::SetTitle { title })
    }

    #[inline]
    pub fn set_scale_factor(&mut self, scale_factor: f64) -> &mut Self {
        self.push(WindowCommand::SetScaleFactor { scale_factor })
    }

    #[inline]
    pub fn set_resolution(
        &mut self,
        logical_resolution: (f32, f32),
        scale_factor: f64,
    ) -> &mut Self {
        self.push(WindowCommand::SetResolution {
            logical_resolution,
            scale_factor,
        })
    }

    #[inline]
    #[doc(alias = "set_vsync")]
    pub fn set_present_mode(&mut self, present_mode: PresentMode) -> &mut Self {
        self.push(WindowCommand::SetPresentMode { present_mode })
    }

    #[inline]
    pub fn set_resizable(&mut self, resizable: bool) -> &mut Self {
        self.push(WindowCommand::SetResizable { resizable })
    }

    #[inline]
    pub fn set_decorations(&mut self, decorations: bool) -> &mut Self {
        self.push(WindowCommand::SetDecorations { decorations })
    }

    #[inline]
    pub fn set_cursor_lock_mode(&mut self, locked: bool) -> &mut Self {
        self.push(WindowCommand::SetCursorLockMode { locked })
    }

    #[inline]
    pub fn set_cursor_icon(&mut self, icon: CursorIcon) -> &mut Self {
        self.push(WindowCommand::SetCursorIcon { icon })
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, visible: bool) -> &mut Self {
        self.push(WindowCommand::SetCursorVisibility { visible })
    }

    #[inline]
    pub fn set_cursor_position(&mut self, position: Vec2) -> &mut Self {
        self.push(WindowCommand::SetCursorPosition { position })
    }

    /// Sets the window to maximized.
    #[inline]
    pub fn set_maximized(&mut self, maximized: bool) -> &mut Self {
        self.push(WindowCommand::SetMaximized { maximized })
    }

    /// Sets the window to minimized or back.
    ///
    /// # Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - Wayland: Un-minimize is unsupported.
    #[inline]
    pub fn set_minimized(&mut self, minimized: bool) -> &mut Self {
        self.push(WindowCommand::SetMinimized { minimized })
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
        self.push(WindowCommand::SetPosition { position })
    }

    /// Modifies the minimum and maximum window bounds for resizing in logical pixels.
    #[inline]
    pub fn set_resize_constraints(
        &mut self,
        resize_constraints: WindowResizeConstraints,
    ) -> &mut Self {
        self.push(WindowCommand::SetResizeConstraints { resize_constraints })
    }
}

pub trait CommandsExt<'w, 's> {
    fn window<'a>(&'a mut self, window: Entity) -> WindowCommands<'w, 's, 'a>;
}

impl<'w, 's> CommandsExt<'w, 's> for Commands<'w, 's> {
    #[track_caller]
    fn window<'a>(&'a mut self, window: Entity) -> WindowCommands<'w, 's, 'a> {
        // currently there is no way of checking if an entity exists from `Commands`
        // some function like `exists` or `contains` should get added to `Commands`
        WindowCommands {
            window,
            commands: self,
        }
    }
}

#[cfg(test)]
mod test {
    use bevy_ecs::{
        entity::Entity,
        event::{EventReader, Events, ManualEventReader},
        prelude::{Added, World},
        schedule::{Stage, SystemStage},
        system::{CommandQueue, Commands, Query},
    };

    use crate::{CommandsExt, WindowCommandQueued, WindowDescriptor};

    #[test]
    fn test_commands() {
        let mut world = World::new();

        let mut ev_commands = ManualEventReader::<WindowCommandQueued>::default();
        world.init_resource::<Events<WindowCommandQueued>>();

        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let first_window = commands.spawn().id();
        commands
            .window(first_window)
            .set_resizable(true)
            .set_decorations(false);

        let second_window = commands.spawn().id();
        commands
            .window(second_window)
            .set_title("some title".to_string());

        queue.apply(&mut world);

        let events = world.get_resource::<Events<WindowCommandQueued>>().unwrap();
        let commands = ev_commands
            .iter(events)
            .map(|WindowCommandQueued { window, command }| {
                format!("window = {window:?}, command = {command:?}")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            commands,
            vec![
                "window = 0v0, command = SetResizable { resizable: true }",
                "window = 0v0, command = SetDecorations { decorations: false }",
                "window = 1v0, command = SetTitle { title: \"some title\" }",
            ]
        );
    }

    fn spawn_windows(mut commands: Commands) {
        let first_window = commands.spawn().id();
        commands
            .window(first_window)
            .set_resizable(true)
            .set_decorations(false);

        let second_window = commands.spawn().id();
        commands
            .window(second_window)
            .set_title("some title".to_string());
    }

    fn check_events(mut events: EventReader<WindowCommandQueued>) {
        for WindowCommandQueued { window, command } in events.iter() {
            println!("window = {window:?}, command = {command:?}");
        }
    }

    #[test]
    fn test_system() {
        let mut world = World::new();
        world.init_resource::<Events<WindowCommandQueued>>();

        let mut stage = SystemStage::single_threaded();
        stage.add_system(spawn_windows);
        stage.add_system(check_events);

        stage.run(&mut world);
        stage.run(&mut world);
    }

    fn spawn_window_descriptors(mut commands: Commands) {
        let first_window = commands.spawn().insert(WindowDescriptor::default()).id();
        commands
            .window(first_window)
            .set_resizable(true)
            .set_decorations(false);

        let second_window = commands.spawn().insert(WindowDescriptor::default()).id();
        commands
            .window(second_window)
            .set_title("some title".to_string());
    }

    fn check_window_descriptors(
        descriptors: Query<(Entity, &WindowDescriptor), Added<WindowDescriptor>>,
    ) {
        for (entity, descriptor) in descriptors.iter() {
            println!("entity = {entity:?}, descriptor = {descriptor:?}");
        }
    }

    #[test]
    fn test_window_descriptors() {
        let mut world = World::new();
        world.init_resource::<Events<WindowCommandQueued>>();

        let mut startup_stage =
            SystemStage::single_threaded().with_system(spawn_window_descriptors);

        startup_stage.run(&mut world);

        let mut stage = SystemStage::single_threaded()
            .with_system(check_window_descriptors)
            .with_system(check_events);

        stage.run(&mut world);
        stage.run(&mut world);

        world.spawn().insert(WindowDescriptor::default());

        stage.run(&mut world);
    }
}
