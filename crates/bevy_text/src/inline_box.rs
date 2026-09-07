use bevy_ecs::component::Component;
pub use parley::InlineBoxKind;

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
