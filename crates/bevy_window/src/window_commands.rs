use bevy_ecs::{
    entity::Entity,
    event::Events,
    prelude::World,
    system::{Command, Commands},
};
use bevy_math::{DVec2, IVec2, Vec2};

use crate::{
    CursorIcon, PresentMode, RawWindowHandleWrapper, Window,
    WindowDescriptor, WindowMode, WindowResizeConstraints,
};

// TODO: Docs
pub trait WindowCommandsExtension<'w, 's> {
    // TODO: Docs
    fn window<'a>(&'a mut self, entity: Entity) -> WindowCommands<'w, 's, 'a>;
    // TODO: Docs
    fn spawn_window<'a>(&'a mut self, descriptor: WindowDescriptor) -> WindowCommands<'w, 's, 'a>;
}

impl<'w, 's> WindowCommandsExtension<'w, 's> for Commands<'w, 's> {
    // TODO: Docs
    /// Gives you windowcommands for an entity
    fn window<'a>(&'a mut self, entity: Entity) -> WindowCommands<'w, 's, 'a> {
        assert!(
            self.has_entity(entity),
            "Attempting to create an WindowCommands for entity {:?}, which doesn't exist.",
            entity
        );

        WindowCommands {
            entity,
            commands: self,
        }
    }

    // TODO: Docs
    /// Spawns and entity, then gives you window-commands for that entity
    fn spawn_window<'a>(&'a mut self, descriptor: WindowDescriptor) -> WindowCommands<'w, 's, 'a> {
        let entity = self.spawn().id();

        self.add(CreateWindowCommand { entity, descriptor });

        WindowCommands {
            entity,
            commands: self,
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct CreateWindowCommand {
    pub entity: Entity,
    pub descriptor: WindowDescriptor,
}

impl Command for CreateWindowCommand {
    fn write(self, world: &mut World) {
        // Make sure we only create new windows on entities that has none
        if let None = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<CreateWindowCommand>>();
            event.send(self);
        } else {
            panic!("Can't create a window on an entity that already has a Window");
        }

        // match world.resource_mut::<Events<CreateWindowCommand>>() {
        //     mut create_window_event => {
        //         create_window_event.send(self);
        //     }
        //     _ => {
        //             panic!(
        //                 "Could not send CreateWindow event as the Event<CreateWindow> has not been created"
        //             );
        //         }
        // }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetWindowModeCommand {
    pub entity: Entity,
    pub mode: WindowMode,
    pub resolution: (u32, u32),
}

impl Command for SetWindowModeCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetWindowModeCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetTitleCommand {
    pub entity: Entity,
    pub title: String,
}

impl Command for SetTitleCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetTitleCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetScaleFactorOverrideCommand {
    pub entity: Entity,
    pub scale_factor: Option<f64>,
}

impl Command for SetScaleFactorOverrideCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetScaleFactorOverrideCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetResolutionCommand {
    pub entity: Entity,
    pub logical_resolution: (f32, f32),
    pub scale_factor: f64,
}

impl Command for SetResolutionCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetResolutionCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetPresentModeCommand {
    pub entity: Entity,
    pub present_mode: PresentMode,
}

impl Command for SetPresentModeCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetPresentModeCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetResizableCommand {
    pub entity: Entity,
    pub resizable: bool,
}

impl Command for SetResizableCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetResizableCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetDecorationsCommand {
    pub entity: Entity,
    pub decorations: bool,
}

impl Command for SetDecorationsCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetDecorationsCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetCursorLockModeCommand {
    pub entity: Entity,
    pub locked: bool,
}

impl Command for SetCursorLockModeCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetCursorLockModeCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetCursorIconCommand {
    pub entity: Entity,
    pub icon: CursorIcon,
}

impl Command for SetCursorIconCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetCursorIconCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetCursorVisibilityCommand {
    pub entity: Entity,
    pub visible: bool,
}

impl Command for SetCursorVisibilityCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetCursorVisibilityCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetCursorPositionCommand {
    pub entity: Entity,
    pub position: DVec2,
}

impl Command for SetCursorPositionCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetCursorPositionCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetMaximizedCommand {
    pub entity: Entity,
    pub maximized: bool,
}

impl Command for SetMaximizedCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetMaximizedCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetMinimizedCommand {
    pub entity: Entity,
    pub minimized: bool,
}

impl Command for SetMinimizedCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetMinimizedCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetPositionCommand {
    pub entity: Entity,
    pub position: IVec2,
}

impl Command for SetPositionCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetPositionCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct SetResizeConstraintsCommand {
    pub entity: Entity,
    pub resize_constraints: WindowResizeConstraints,
}

impl Command for SetResizeConstraintsCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<SetResizeConstraintsCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
#[derive(Debug)]
pub struct CloseWindowCommand {
    pub entity: Entity,
}

impl Command for CloseWindowCommand {
    fn write(self, world: &mut World) {
        if let Some(_) = world.get::<Window>(self.entity) {
            let mut event = world.resource_mut::<Events<CloseWindowCommand>>();
            event.send(self);
        } else {
            panic!("Trying to enact window commands on an entity without a window-component");
        }
    }
}

// TODO: Docs
pub struct WindowCommands<'w, 's, 'a> {
    entity: Entity,
    commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> WindowCommands<'w, 's, 'a> {
    // TODO: Update documentation
    /// Returns the [`Entity`] id of the entity.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// fn my_system(mut commands: Commands) {
    ///     let entity_id = commands.spawn().id();
    /// }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity
    }

    pub fn create_window(&mut self, window_desciptor: WindowDescriptor) -> &mut Self {
        self.commands.add(CreateWindowCommand {
            entity: self.entity,
            descriptor: window_desciptor,
        });
        self
    }

    #[inline]
    pub fn set_maximized(&mut self, maximized: bool) -> &mut Self {
        self.commands.add(SetMaximizedCommand {
            entity: self.entity,
            maximized,
        });
        self
    }

    /// Sets the window to minimized or back.
    ///
    /// # Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - Wayland: Un-minimize is unsupported.
    #[inline]
    pub fn set_minimized(&mut self, minimized: bool) -> &mut Self {
        self.commands.add(SetMinimizedCommand {
            entity: self.entity,
            minimized,
        });
        self
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
        self.commands.add(SetPositionCommand {
            entity: self.entity,
            position,
        });
        self
    }

    /// Modifies the minimum and maximum window bounds for resizing in logical pixels.
    #[inline]
    pub fn set_resize_constraints(
        &mut self,
        resize_constraints: WindowResizeConstraints,
    ) -> &mut Self {
        self.commands.add(SetResizeConstraintsCommand {
            entity: self.entity,
            resize_constraints,
        });
        self
    }

    /// Close the operating system window corresponding to this [`Window`].  
    /// This will also lead to this [`Window`] being removed from the
    /// [`Windows`] resource.
    ///
    /// If the default [`WindowPlugin`] is used, when no windows are
    /// open, the [app will exit](bevy_app::AppExit).  
    /// To disable this behaviour, set `exit_on_all_closed` on the [`WindowPlugin`]
    /// to `false`
    ///
    /// [`Windows`]: crate::Windows
    /// [`WindowPlugin`]: crate::WindowPlugin
    pub fn close(&mut self) -> &mut Self {
        self.commands.add(CloseWindowCommand {
            entity: self.entity,
        });
        self
    }

    pub fn set_title(&mut self, title: String) -> &mut Self {
        self.commands.add(SetTitleCommand {
            entity: self.entity,
            title,
        });
        self
    }

    #[allow(clippy::float_cmp)]
    pub fn set_resolution(&mut self, width: f32, height: f32, scale_factor: f64) -> &mut Self {
        // TODO: Should not send the command if new is the same as old?
        // if self.requested_width == width && self.requested_height == height {
        //     return;
        // // }
        self.commands.add(SetResolutionCommand {
            entity: self.entity,
            logical_resolution: (width, height),
            scale_factor,
        });

        self
    }

    /// Override the os-reported scaling factor
    #[allow(clippy::float_cmp)]
    pub fn set_scale_factor_override(&mut self, scale_factor: Option<f64>) -> &mut Self {
        // TODO: Not do anything if new is same as old?
        // if self.scale_factor_override == scale_factor {
        //     return;
        // }

        // self.scale_factor_override = scale_factor;
        self.commands.add(SetScaleFactorOverrideCommand {
            entity: self.entity,
            scale_factor,
        });

        self

        // TODO: Sending scale-factor event should also update the resolution
        // self.commands.add(WindowCommand::SetResolution {
        //     logical_resolution: (self.requested_width, self.requested_height),
        //     scale_factor: self.scale_factor(),
        // });
    }

    #[inline]
    #[doc(alias = "set_vsync")]
    pub fn set_present_mode(&mut self, present_mode: PresentMode) -> &mut Self {
        self.commands.add(SetPresentModeCommand {
            entity: self.entity,
            present_mode,
        });

        self
    }

    pub fn set_resizable(&mut self, resizable: bool) -> &mut Self {
        self.commands.add(SetResizableCommand {
            entity: self.entity,
            resizable,
        });
        self
    }

    pub fn set_decorations(&mut self, decorations: bool) -> &mut Self {
        self.commands.add(SetDecorationsCommand {
            entity: self.entity,
            decorations,
        });
        self
    }

    pub fn set_cursor_lock_mode(&mut self, lock_mode: bool) -> &mut Self {
        self.commands.add(SetCursorLockModeCommand {
            entity: self.entity,
            locked: lock_mode,
        });
        self
    }

    pub fn set_cursor_visibility(&mut self, visibile_mode: bool) -> &mut Self {
        self.commands.add(SetCursorVisibilityCommand {
            entity: self.entity,
            visible: visibile_mode,
        });
        self
    }

    pub fn set_cursor_icon(&mut self, icon: CursorIcon) -> &mut Self {
        self.commands.add(SetCursorIconCommand {
            entity: self.entity,
            icon,
        });
        self
    }

    // TODO: This should be a resource that calculates this?
    /// The current mouse position, in logical pixels, taking into account the screen scale factor.
    // #[inline]
    // #[doc(alias = "mouse position")]
    // pub fn cursor_position(&self) -> Option<Vec2> {
    //     self.physical_cursor_position
    //         .map(|p| (p / self.scale_factor()).as_vec2())
    // }

    pub fn set_cursor_position(&mut self, position: DVec2) -> &mut Self {
        self.commands.add(SetCursorPositionCommand {
            entity: self.entity,
            position,
        });
        self
    }

    // TODO: Backend should add or remove WindowFocused marker component
    // #[allow(missing_docs)]
    // #[inline]
    // pub fn update_focused_status_from_backend(&mut self, focused: bool) {
    //     self.focused = focused;
    // }

    // TODO: What to do with this?
    // #[allow(missing_docs)]
    // #[inline]
    // pub fn update_cursor_physical_position_from_backend(&mut self, cursor_position: Option<DVec2>) {
    //     self.physical_cursor_position = cursor_position;
    // }

    pub fn set_mode(&mut self, mode: WindowMode, resolution: (u32, u32)) -> &mut Self {
        self.commands.add(SetWindowModeCommand {
            entity: self.entity,
            mode,
            resolution,
        });

        self
    }

    // TODO: What to do with this?
    // pub fn raw_window_handle(&self) -> RawWindowHandleWrapper {
    //     self.raw_window_handle.clone()
    // }
}
