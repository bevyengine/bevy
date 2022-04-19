use bevy_ecs::{
    entity::Entity,
    event::Events,
    prelude::World,
    system::{
        ReadOnlySystemParamFetch, SystemMeta, SystemParam, SystemParamFetch, SystemParamState,
    },
};
use bevy_math::{IVec2, Vec2};

use crate::{
    CursorIcon, PresentMode, WindowCommand, WindowCommandQueued, WindowMode,
    WindowResizeConstraints,
};

#[derive(Debug, Default)]
pub struct WindowCommandQueue {
    commands: Vec<(Entity, WindowCommand)>,
}

impl WindowCommandQueue {
    #[inline]
    pub fn push(&mut self, window: Entity, command: WindowCommand) {
        self.commands.push((window, command));
    }

    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        let mut events = world
            .get_resource_mut::<Events<WindowCommandQueued>>()
            .unwrap();
        for (window, command) in self.commands.drain(..) {
            events.send(WindowCommandQueued { window, command });
        }
    }
}

pub struct WindowCommands<'s> {
    queue: &'s mut WindowCommandQueue,
}

impl<'s> WindowCommands<'s> {
    pub fn new(queue: &'s mut WindowCommandQueue) -> Self {
        Self { queue }
    }

    #[inline]
    pub fn push(&mut self, window: Entity, command: WindowCommand) {
        self.queue.push(window, command);
    }

    #[inline]
    pub fn set_window_mode(&mut self, window: Entity, mode: WindowMode, resolution: (u32, u32)) {
        self.queue
            .push(window, WindowCommand::SetWindowMode { mode, resolution });
    }

    #[inline]
    pub fn set_title(&mut self, window: Entity, title: String) {
        self.queue.push(window, WindowCommand::SetTitle { title });
    }

    #[inline]
    pub fn set_scale_factor(&mut self, window: Entity, scale_factor: f64) {
        self.queue
            .push(window, WindowCommand::SetScaleFactor { scale_factor });
    }

    #[inline]
    pub fn set_resolution(
        &mut self,
        window: Entity,
        logical_resolution: (f32, f32),
        scale_factor: f64,
    ) {
        self.queue.push(
            window,
            WindowCommand::SetResolution {
                logical_resolution,
                scale_factor,
            },
        );
    }

    #[inline]
    #[doc(alias = "set_vsync")]
    pub fn set_present_mode(&mut self, window: Entity, present_mode: PresentMode) {
        self.queue
            .push(window, WindowCommand::SetPresentMode { present_mode });
    }

    #[inline]
    pub fn set_resizable(&mut self, window: Entity, resizable: bool) {
        self.queue
            .push(window, WindowCommand::SetResizable { resizable });
    }

    #[inline]
    pub fn set_decorations(&mut self, window: Entity, decorations: bool) {
        self.queue
            .push(window, WindowCommand::SetDecorations { decorations });
    }

    #[inline]
    pub fn set_cursor_lock_mode(&mut self, window: Entity, locked: bool) {
        self.queue
            .push(window, WindowCommand::SetCursorLockMode { locked });
    }

    #[inline]
    pub fn set_cursor_icon(&mut self, window: Entity, icon: CursorIcon) {
        self.queue
            .push(window, WindowCommand::SetCursorIcon { icon });
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, window: Entity, visible: bool) {
        self.queue
            .push(window, WindowCommand::SetCursorVisibility { visible });
    }

    #[inline]
    pub fn set_cursor_position(&mut self, window: Entity, position: Vec2) {
        self.queue
            .push(window, WindowCommand::SetCursorPosition { position });
    }

    /// Sets the window to maximized.
    #[inline]
    pub fn set_maximized(&mut self, window: Entity, maximized: bool) {
        self.queue
            .push(window, WindowCommand::SetMaximized { maximized });
    }

    /// Sets the window to minimized or back.
    ///
    /// # Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - Wayland: Un-minimize is unsupported.
    #[inline]
    pub fn set_minimized(&mut self, window: Entity, minimized: bool) {
        self.queue
            .push(window, WindowCommand::SetMinimized { minimized });
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
    pub fn set_position(&mut self, window: Entity, position: IVec2) {
        self.queue
            .push(window, WindowCommand::SetPosition { position });
    }

    /// Modifies the minimum and maximum window bounds for resizing in logical pixels.
    #[inline]
    pub fn set_resize_constraints(
        &mut self,
        window: Entity,
        resize_constraints: WindowResizeConstraints,
    ) {
        self.queue.push(
            window,
            WindowCommand::SetResizeConstraints { resize_constraints },
        );
    }
}

impl<'s> SystemParam for WindowCommands<'s> {
    type Fetch = WindowCommandQueue;
}

unsafe impl ReadOnlySystemParamFetch for WindowCommandQueue {}

unsafe impl SystemParamState for WindowCommandQueue {
    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Default::default()
    }

    fn apply(&mut self, world: &mut World) {
        self.apply(world);
    }
}

impl<'w, 's> SystemParamFetch<'w, 's> for WindowCommandQueue {
    type Item = WindowCommands<'s>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        _system_meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        WindowCommands::new(state)
    }
}

#[cfg(test)]
mod test {
    use bevy_ecs::{
        entity::Entity,
        event::{EventReader, Events, ManualEventReader},
        prelude::{Added, World},
        schedule::{Stage, SystemStage},
        system::{Commands, Query},
    };

    use crate::{WindowCommandQueue, WindowCommandQueued, WindowCommands, WindowDescriptor};

    #[test]
    fn test_commands() {
        let mut world = World::new();

        let mut ev_commands = ManualEventReader::<WindowCommandQueued>::default();
        world.init_resource::<Events<WindowCommandQueued>>();

        let mut queue = WindowCommandQueue::default();
        let mut commands = WindowCommands::new(&mut queue);

        let first_window = world.spawn().id();
        commands.set_resizable(first_window, true);
        commands.set_decorations(first_window, false);

        let second_window = world.spawn().id();
        commands.set_title(second_window, "some title".to_string());

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

    fn spawn_windows(mut commands: Commands, mut window_commands: WindowCommands) {
        let first_window = commands.spawn().id();
        window_commands.set_resizable(first_window, true);
        window_commands.set_decorations(first_window, false);

        let second_window = commands.spawn().id();
        window_commands.set_title(second_window, "some title".to_string());
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

    fn spawn_window_descriptors(mut commands: Commands, mut window_commands: WindowCommands) {
        let first_window = commands.spawn().insert(WindowDescriptor::default()).id();
        window_commands.set_resizable(first_window, true);
        window_commands.set_decorations(first_window, false);

        let second_window = commands.spawn().insert(WindowDescriptor::default()).id();
        window_commands.set_title(second_window, "some title".to_string());
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
