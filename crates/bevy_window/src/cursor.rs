use bevy_reflect::{prelude::ReflectDefault, Reflect};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// The icon to display for a [`Window`](crate::window::Window)'s [`Cursor`](crate::window::Cursor).
///
/// Examples of all of these cursors can be found [here](https://www.w3schools.com/cssref/playit.php?filename=playcss_cursor&preval=crosshair).
/// This `enum` is simply a copy of a similar `enum` found in [`winit`](https://docs.rs/winit/latest/winit/window/enum.CursorIcon.html).
/// `winit`, in turn, mostly copied cursor types available in the browser.
#[non_exhaustive]
#[derive(Default, Debug, Hash, PartialEq, Eq, Clone, Copy, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub enum CursorIcon {
    /// The platform-dependent default cursor. Often rendered as arrow.
    #[default]
    Default,

    /// A context menu is available for the object under the cursor. Often
    /// rendered as an arrow with a small menu-like graphic next to it.
    ContextMenu,

    /// Help is available for the object under the cursor. Often rendered as a
    /// question mark or a balloon.
    Help,

    /// The cursor is a pointer that indicates a link. Often rendered as the
    /// backside of a hand with the index finger extended.
    Pointer,

    /// A progress indicator. The program is performing some processing, but is
    /// different from [`CursorIcon::Wait`] in that the user may still interact
    /// with the program.
    Progress,

    /// Indicates that the program is busy and the user should wait. Often
    /// rendered as a watch or hourglass.
    Wait,

    /// Indicates that a cell or set of cells may be selected. Often rendered as
    /// a thick plus-sign with a dot in the middle.
    Cell,

    /// A simple crosshair (e.g., short line segments resembling a "+" sign).
    /// Often used to indicate a two dimensional bitmap selection mode.
    Crosshair,

    /// Indicates text that may be selected. Often rendered as an I-beam.
    Text,

    /// Indicates vertical-text that may be selected. Often rendered as a
    /// horizontal I-beam.
    VerticalText,

    /// Indicates an alias of/shortcut to something is to be created. Often
    /// rendered as an arrow with a small curved arrow next to it.
    Alias,

    /// Indicates something is to be copied. Often rendered as an arrow with a
    /// small plus sign next to it.
    Copy,

    /// Indicates something is to be moved.
    Move,

    /// Indicates that the dragged item cannot be dropped at the current cursor
    /// location. Often rendered as a hand or pointer with a small circle with a
    /// line through it.
    NoDrop,

    /// Indicates that the requested action will not be carried out. Often
    /// rendered as a circle with a line through it.
    NotAllowed,

    /// Indicates that something can be grabbed (dragged to be moved). Often
    /// rendered as the backside of an open hand.
    Grab,

    /// Indicates that something is being grabbed (dragged to be moved). Often
    /// rendered as the backside of a hand with fingers closed mostly out of
    /// view.
    Grabbing,

    /// The east border to be moved.
    EResize,

    /// The north border to be moved.
    NResize,

    /// The north-east corner to be moved.
    NeResize,

    /// The north-west corner to be moved.
    NwResize,

    /// The south border to be moved.
    SResize,

    /// The south-east corner to be moved.
    SeResize,

    /// The south-west corner to be moved.
    SwResize,

    /// The west border to be moved.
    WResize,

    /// The east and west borders to be moved.
    EwResize,

    /// The south and north borders to be moved.
    NsResize,

    /// The north-east and south-west corners to be moved.
    NeswResize,

    /// The north-west and south-east corners to be moved.
    NwseResize,

    /// Indicates that the item/column can be resized horizontally. Often
    /// rendered as arrows pointing left and right with a vertical bar
    /// separating them.
    ColResize,

    /// Indicates that the item/row can be resized vertically. Often rendered as
    /// arrows pointing up and down with a horizontal bar separating them.
    RowResize,

    /// Indicates that the something can be scrolled in any direction. Often
    /// rendered as arrows pointing up, down, left, and right with a dot in the
    /// middle.
    AllScroll,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "+" in the center of the glass.
    ZoomIn,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "-" in the center of the glass.
    ZoomOut,
}
