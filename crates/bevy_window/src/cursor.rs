/// The icon to display for a window's cursor.
///
/// Examples of all of these cursors can be found [here](https://www.w3schools.com/cssref/playit.asp?filename=playcss_cursor).
/// This `enum` is simply a copy of a similar `enum` found in [`winit`](https://docs.rs/winit/latest/winit/window/enum.CursorIcon.html).
/// `winit`, in turn, mostly copied cursor types available in the browser.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum CursorIcon {
    /// The platform-dependent default cursor.
    Default,
    /// A simple crosshair.   
    Crosshair,
    /// A hand (often used to indicate links in web browsers).    
    Hand,
    /// An arrow. This is the default cursor on most systems.    
    Arrow,
    /// Indicates something is to be moved.    
    Move,
    /// Indicates text that may be selected or edited.    
    Text,
    /// Program busy indicator.    
    Wait,
    /// Help indicator (often rendered as a "?")    
    Help,
    /// Progress indicator. Shows that processing is being done.
    ///
    /// But in contrast with "Wait" the user may still interact with the program.
    /// Often rendered as a spinning beach ball, or an arrow with a watch or hourglass.    
    Progress,
    /// Cursor showing that something cannot be done.    
    NotAllowed,
    /// Indicates that a context menu is available.
    ContextMenu,
    /// Indicates that a cell (or set of cells) may be selected.
    Cell,
    /// Indicates vertical text that may be selected or edited.
    VerticalText,
    /// Indicates that an alias of something is to be created.
    Alias,
    /// Indicates something is to be copied.
    Copy,
    /// Indicates that the dragged item cannot be dropped here.
    NoDrop,
    /// Indicates that something can be grabbed.
    Grab,
    /// Indicates that something is grabbed.
    Grabbing,
    /// Indicates that the user can scroll by dragging the mouse.
    AllScroll,
    /// Indicates that the user can zoom in.
    ZoomIn,
    /// Indicates that the user can zoom out.
    ZoomOut,
    /// Indicates that an edge of a box is to be moved right (east).
    EResize,
    /// Indicates that an edge of a box is to be moved up (north).
    NResize,
    /// Indicates that an edge of a box is to be moved up and right (north/east).
    NeResize,
    /// indicates that an edge of a box is to be moved up and left (north/west).
    NwResize,
    /// Indicates that an edge of a box is to be moved down (south).
    SResize,
    /// The cursor indicates that an edge of a box is to be moved down and right (south/east).
    SeResize,
    /// The cursor indicates that an edge of a box is to be moved down and left (south/west).
    SwResize,
    /// Indicates that an edge of a box is to be moved left (west).
    WResize,
    /// Indicates a bidirectional resize cursor.
    EwResize,
    /// Indicates a bidirectional resize cursor.
    NsResize,
    /// Indicates a bidirectional resize cursor.
    NeswResize,
    /// Indicates a bidirectional resize cursor.
    NwseResize,
    /// Indicates that a column can be resized horizontally.
    ColResize,
    /// Indicates that the row can be resized vertically.
    RowResize,
}

impl From<CursorIcon> for winit::window::CursorIcon {
    fn from(cursor_icon: CursorIcon) -> Self {
        match cursor_icon {
            CursorIcon::Default => winit::window::CursorIcon::Default,
            CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
            CursorIcon::Hand => winit::window::CursorIcon::Hand,
            CursorIcon::Arrow => winit::window::CursorIcon::Arrow,
            CursorIcon::Move => winit::window::CursorIcon::Move,
            CursorIcon::Text => winit::window::CursorIcon::Text,
            CursorIcon::Wait => winit::window::CursorIcon::Wait,
            CursorIcon::Help => winit::window::CursorIcon::Help,
            CursorIcon::Progress => winit::window::CursorIcon::Progress,
            CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
            CursorIcon::ContextMenu => winit::window::CursorIcon::ContextMenu,
            CursorIcon::Cell => winit::window::CursorIcon::Cell,
            CursorIcon::VerticalText => winit::window::CursorIcon::VerticalText,
            CursorIcon::Alias => winit::window::CursorIcon::Alias,
            CursorIcon::Copy => winit::window::CursorIcon::Copy,
            CursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
            CursorIcon::Grab => winit::window::CursorIcon::Grab,
            CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
            CursorIcon::AllScroll => winit::window::CursorIcon::AllScroll,
            CursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
            CursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
            CursorIcon::EResize => winit::window::CursorIcon::EResize,
            CursorIcon::NResize => winit::window::CursorIcon::NResize,
            CursorIcon::NeResize => winit::window::CursorIcon::NeResize,
            CursorIcon::NwResize => winit::window::CursorIcon::NwResize,
            CursorIcon::SResize => winit::window::CursorIcon::SResize,
            CursorIcon::SeResize => winit::window::CursorIcon::SeResize,
            CursorIcon::SwResize => winit::window::CursorIcon::SwResize,
            CursorIcon::WResize => winit::window::CursorIcon::WResize,
            CursorIcon::EwResize => winit::window::CursorIcon::EwResize,
            CursorIcon::NsResize => winit::window::CursorIcon::NsResize,
            CursorIcon::NeswResize => winit::window::CursorIcon::NeswResize,
            CursorIcon::NwseResize => winit::window::CursorIcon::NwseResize,
            CursorIcon::ColResize => winit::window::CursorIcon::ColResize,
            CursorIcon::RowResize => winit::window::CursorIcon::RowResize,
        }
    }
}
