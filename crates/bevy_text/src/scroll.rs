//! EditableText scrolling logic
//!
//! - [`TextViewport`] is a rectangle aligned to the text layout's axis representing
//! the user's view of the text layout.
//! - Coordinates are in text layout space, increasing right and downwards.
//! - If the text layout is smaller than the viewport on an axis, the viewport is
//!  given an offset of zero on that the axis.
//! - An origin-size representation is used because the size is generally fixed.
//! This avoids floating point error accumulation that might happen with min-max coords.
//!
//! # Scrolling rules
//!
//! - The text viewport is controlled exclusively through `TextEdit`s.
//! - Displacement scrolling is continuous and unquantized.
//! - Scrolling to a point moves each axis by the minimum amount needed to make
//!   that point visible.
//! - Scrolling by lines moves by the distance between visual-line starts.
//!   Fractional amounts interpolate across the next line interval in the scroll
//!   direction. This supports wrapped and variable-height lines without
//!   snapping the viewport to line bounds.
//!
//! Text and keyboard edits reveal the caret, including edits that do not change
//! the editor state. Pointer-driven edits and explicit scroll edits do not.
//! Caret reveal follows these rules:
//!
//! - Normalized horizontal and vertical margins inset a caret reveal region from every
//!   viewport edge. Caret movement within this region leaves the viewport unchanged.
//! - If the caret leaves the caret reveal region horizontally, the viewport scrolls
//!   sideways by the smallest amount that brings it back inside, keeping one
//!   `0`-advance of space visible from the caret position onward to leave room for the
//!   next typed character.
//! - Vertically, the viewport scrolls to reveal Parley's caret rectangle,
//!   which spans the whole visual line. The margin-aware offset is clamped
//!   toward the next visual-line start in the scroll direction, so a caret
//!   that moved up or down leaves the viewport aligned to a line in that direction.
//! - If the caret or its visual line is too large to fit inside the caret
//!   reveal region on an axis, then the viewport is centered on it on that axis instead.
//! - The viewport never scrolls outside the layout bounds (and when the
//!   viewport is larger than the layout on an axis, it does not scroll on
//!   that axis at all), even when that leaves the caret closer to the
//!   viewport edge than the margin requests.

use bevy_math::Vec2;
use bevy_reflect::Reflect;

/// The region of the editable text layout visible to the user.
///
/// Scrolling changes the offset, size depends on the layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Reflect)]
pub struct TextViewport {
    /// The top-left corner of the text viewport in text-layout coordinates.
    offset: Vec2,
    /// The size of the viewport in text-layout coordinates.
    size: Vec2,
}
