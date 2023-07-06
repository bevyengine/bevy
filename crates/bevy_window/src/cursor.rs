use bevy_reflect::{prelude::ReflectDefault, Reflect};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// The icon to display for a [`Window`](crate::window::Window)'s [`Cursor`](crate::window::Cursor).
///
/// Examples of all of these cursors can be found [here](https://www.w3schools.com/cssref/playit.php?filename=playcss_cursor&preval=crosshair).
/// This `enum` is simply a copy of a similar `enum` found in [`winit`](https://docs.rs/winit/latest/winit/window/enum.CursorIcon.html).
/// `winit`, in turn, mostly copied cursor types available in the browser.
#[derive(Default, Debug, Hash, PartialEq, Eq, Clone, Copy, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub enum CursorIcon {
    /// The platform-dependent default cursor.
    #[default]
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
