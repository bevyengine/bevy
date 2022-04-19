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
    CursorIcon, PresentMode, WindowCommand, WindowCommandsQueued, WindowMode,
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
            .get_resource_mut::<Events<WindowCommandsQueued>>()
            .unwrap();
        events.send(WindowCommandsQueued {
            commands: self.commands.drain(..).collect(),
        });
    }
}

pub struct WindowCommands<'s> {
    queue: &'s mut WindowCommandQueue,
}

impl<'s> WindowCommands<'s> {
    pub fn new(queue: &'s mut WindowCommandQueue) -> Self {
        Self { queue }
    }

    pub fn set_window_mode(&mut self, window: Entity, mode: WindowMode, resolution: (u32, u32)) {
        self.queue
            .push(window, WindowCommand::SetWindowMode { mode, resolution });
    }

    pub fn set_title(&mut self, window: Entity, title: String) {
        self.queue.push(window, WindowCommand::SetTitle { title });
    }

    pub fn set_scale_factor(&mut self, window: Entity, scale_factor: f64) {
        self.queue
            .push(window, WindowCommand::SetScaleFactor { scale_factor });
    }

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

    pub fn set_present_mode(&mut self, window: Entity, present_mode: PresentMode) {
        self.queue
            .push(window, WindowCommand::SetPresentMode { present_mode });
    }

    pub fn set_resizable(&mut self, window: Entity, resizable: bool) {
        self.queue
            .push(window, WindowCommand::SetResizable { resizable });
    }

    pub fn set_decorations(&mut self, window: Entity, decorations: bool) {
        self.queue
            .push(window, WindowCommand::SetDecorations { decorations });
    }

    pub fn set_cursor_lock_mode(&mut self, window: Entity, locked: bool) {
        self.queue
            .push(window, WindowCommand::SetCursorLockMode { locked });
    }

    pub fn set_cursor_icon(&mut self, window: Entity, icon: CursorIcon) {
        self.queue
            .push(window, WindowCommand::SetCursorIcon { icon });
    }

    pub fn set_cursor_visibility(&mut self, window: Entity, visible: bool) {
        self.queue
            .push(window, WindowCommand::SetCursorVisibility { visible });
    }

    pub fn set_cursor_position(&mut self, window: Entity, position: Vec2) {
        self.queue
            .push(window, WindowCommand::SetCursorPosition { position });
    }

    pub fn set_maximized(&mut self, window: Entity, maximized: bool) {
        self.queue
            .push(window, WindowCommand::SetMaximized { maximized });
    }

    pub fn set_minimized(&mut self, window: Entity, minimized: bool) {
        self.queue
            .push(window, WindowCommand::SetMinimized { minimized });
    }

    pub fn set_position(&mut self, window: Entity, position: IVec2) {
        self.queue
            .push(window, WindowCommand::SetPosition { position });
    }

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
        event::{Events, ManualEventReader, EventReader},
        prelude::{World, Added}, schedule::{SystemStage, Stage}, system::{Commands, Query}, entity::Entity,
    };

    use crate::{WindowCommandQueue, WindowCommands, WindowCommandsQueued, WindowDescriptor};

    #[test]
    fn test_commands() {
        let mut world = World::new();

        let mut ev_commands = ManualEventReader::<WindowCommandsQueued>::default();
        world.init_resource::<Events<WindowCommandsQueued>>();

        let mut queue = WindowCommandQueue::default();
        let mut commands = WindowCommands::new(&mut queue);

        let first_window = world.spawn().id();
        commands.set_resizable(first_window, true);
        commands.set_decorations(first_window, false);

        let second_window = world.spawn().id();
        commands.set_title(second_window, "some title".to_string());

        queue.apply(&mut world);

        let events = world
            .get_resource::<Events<WindowCommandsQueued>>()
            .unwrap();
        let commands = ev_commands
            .iter(events)
            .flat_map(|queued| &queued.commands)
            .map(|(window, command)| format!("window = {window:?}, command = {command:?}"))
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

    fn check_events(mut events: EventReader<WindowCommandsQueued>) {
        for (window, command) in events.iter().flat_map(|queued| &queued.commands) {
            println!("window = {window:?}, command = {command:?}");
        }
    }

    #[test]
    fn test_system() {
        let mut world = World::new();
        world.init_resource::<Events<WindowCommandsQueued>>();

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

    fn check_window_descriptors(descriptors: Query<(Entity, &WindowDescriptor), Added<WindowDescriptor>>) {
        for (entity, descriptor) in descriptors.iter() {
            println!("entity = {entity:?}, descriptor = {descriptor:?}");
        }
    }

    #[test]
    fn test_window_descriptors() {
        let mut world = World::new();
        world.init_resource::<Events<WindowCommandsQueued>>();

        let mut stage = SystemStage::single_threaded();
        stage.add_system(spawn_window_descriptors);
        stage.add_system(check_events);
        stage.add_system(check_window_descriptors);
        
        stage.run(&mut world);
        stage.run(&mut world);

        world.spawn().insert(WindowDescriptor::default());
        
        stage.run(&mut world);
    }
}
