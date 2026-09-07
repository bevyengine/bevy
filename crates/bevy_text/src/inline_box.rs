use bevy_ecs::component::Component;
use bevy_reflect::Reflect;

/// A box to be laid out inline with text.
/// Component should only be part of a text tree.
#[derive(Component, Debug, Clone)]
pub struct InlineBox {
    /// Whether the box is in-flow (takes up space in the layout) or out-of-flow (e.g. absolutely positioned or floated)
    pub kind: InlineBoxKind,
    /// The width of the box in pixels
    pub width: f32,
    /// The height of the box in pixels
    pub height: f32,
}

/// Whether a box is in-flow (takes up space in the layout) or out-of-flow (e.g. absolutely positioned)
/// or custom-out-of-flow (line-breaking should yield control flow)
#[derive(PartialEq, Debug, Clone, Copy, Reflect, Default)]
pub enum InlineBoxKind {
    /// `InFlow` boxes take up space in the layout and flow in line with text
    ///
    /// They correspond to `display: inline-block` boxes in CSS.
    #[default]
    InFlow,
    /// `OutOfFlow` boxes are assigned a position as if they were a zero-sized inline box, but
    /// do not take up space in the layout.
    ///
    /// They correspond to `position: absolute` boxes in CSS.
    OutOfFlow,
}

impl From<InlineBoxKind> for parley::InlineBoxKind {
    fn from(inline_box_kind: InlineBoxKind) -> parley::InlineBoxKind {
        match inline_box_kind {
            InlineBoxKind::InFlow => parley::InlineBoxKind::InFlow,
            InlineBoxKind::OutOfFlow => parley::InlineBoxKind::OutOfFlow,
        }
    }
}

impl From<parley::InlineBoxKind> for InlineBoxKind {
    fn from(inline_box_kind: parley::InlineBoxKind) -> InlineBoxKind {
        match inline_box_kind {
            parley::InlineBoxKind::InFlow => InlineBoxKind::InFlow,
            parley::InlineBoxKind::OutOfFlow => InlineBoxKind::OutOfFlow,
            parley::InlineBoxKind::CustomOutOfFlow => {
                unimplemented!("parley::CustomOfFlow is not supported.")
            }
        }
    }
}
