use bevy_utils::Uuid;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(Uuid);

impl WindowId {
    pub fn new() -> Self {
        WindowId(Uuid::new_v4())
    }

    pub fn primary() -> Self {
        WindowId(Uuid::from_u128(0))
    }

    pub fn is_primary(&self) -> bool {
        *self == WindowId::primary()
    }
}

use std::fmt;

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.to_simple().fmt(f)
    }
}

impl Default for WindowId {
    fn default() -> Self {
        WindowId::primary()
    }
}

#[derive(Debug)]
pub struct Window {
    id: WindowId,
    width: u32,
    height: u32,
    title: String,
    vsync: bool,
    resizable: bool,
    decorations: bool,
    cursor_visible: bool,
    cursor_locked: bool,
    mode: WindowMode,
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,
    command_queue: Vec<WindowCommand>,
}

#[derive(Debug)]
pub enum WindowCommand {
    SetWindowMode {
        mode: WindowMode,
        resolution: (u32, u32),
    },
    SetTitle {
        title: String,
    },
    SetResolution {
        width: u32,
        height: u32,
    },
    SetVsync {
        vsync: bool,
    },
    SetResizable {
        resizable: bool,
    },
    SetDecorations {
        decorations: bool,
    },
    SetCursorLockMode {
        locked: bool,
    },
    SetCursorVisibility {
        visible: bool,
    },
    SetCursorPosition {
        x: i32,
        y: i32,
    },
}

/// Defines the way a window is displayed
/// The use_size option that is used in the Fullscreen variant
/// defines whether a videomode is chosen that best fits the width and height
/// in the Window structure, or if these are ignored.
/// E.g. when use_size is set to false the best video mode possible is chosen.
#[derive(Debug, Clone, Copy)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
    Fullscreen { use_size: bool },
}

impl Window {
    pub fn new(id: WindowId, window_descriptor: &WindowDescriptor) -> Self {
        Window {
            id,
            height: window_descriptor.height,
            width: window_descriptor.width,
            title: window_descriptor.title.clone(),
            vsync: window_descriptor.vsync,
            resizable: window_descriptor.resizable,
            decorations: window_descriptor.decorations,
            cursor_visible: window_descriptor.cursor_visible,
            cursor_locked: window_descriptor.cursor_locked,
            mode: window_descriptor.mode,
            #[cfg(target_arch = "wasm32")]
            canvas: window_descriptor.canvas.clone(),
            command_queue: Vec::new(),
        }
    }

    #[inline]
    pub fn id(&self) -> WindowId {
        self.id
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.command_queue
            .push(WindowCommand::SetResolution { width, height });
    }

    #[doc(hidden)]
    pub fn update_resolution_from_backend(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title.to_string();
        self.command_queue.push(WindowCommand::SetTitle { title });
    }

    #[inline]
    pub fn vsync(&self) -> bool {
        self.vsync
    }

    #[inline]
    pub fn set_vsync(&mut self, vsync: bool) {
        self.vsync = vsync;
        self.command_queue.push(WindowCommand::SetVsync { vsync });
    }

    #[inline]
    pub fn resizable(&self) -> bool {
        self.resizable
    }

    pub fn set_resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
        self.command_queue
            .push(WindowCommand::SetResizable { resizable });
    }

    #[inline]
    pub fn decorations(&self) -> bool {
        self.decorations
    }

    pub fn set_decorations(&mut self, decorations: bool) {
        self.decorations = decorations;
        self.command_queue
            .push(WindowCommand::SetDecorations { decorations });
    }

    #[inline]
    pub fn cursor_locked(&self) -> bool {
        self.cursor_locked
    }

    pub fn set_cursor_lock_mode(&mut self, lock_mode: bool) {
        self.cursor_locked = lock_mode;
        self.command_queue
            .push(WindowCommand::SetCursorLockMode { locked: lock_mode });
    }

    #[inline]
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn set_cursor_visibility(&mut self, visibile_mode: bool) {
        self.cursor_visible = visibile_mode;
        self.command_queue.push(WindowCommand::SetCursorVisibility {
            visible: visibile_mode,
        });
    }

    pub fn set_cursor_position(&mut self, x: i32, y: i32) {
        self.command_queue
            .push(WindowCommand::SetCursorPosition { x, y });
    }

    #[inline]
    pub fn mode(&self) -> WindowMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: WindowMode) {
        self.mode = mode;
        self.command_queue.push(WindowCommand::SetWindowMode {
            mode,
            resolution: (self.width, self.height),
        });
    }

    #[inline]
    pub fn drain_commands(&mut self) -> impl Iterator<Item = WindowCommand> + '_ {
        self.command_queue.drain(..)
    }
}

#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub cursor_visible: bool,
    pub cursor_locked: bool,
    pub mode: WindowMode,
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "bevy".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
            resizable: true,
            decorations: true,
            cursor_locked: false,
            cursor_visible: true,
            mode: WindowMode::Windowed,
            #[cfg(target_arch = "wasm32")]
            canvas: None,
        }
    }
}
