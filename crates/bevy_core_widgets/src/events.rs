use bevy_ecs::{event::Event, hierarchy::ChildOf};

/// An event that indicates a change in value of a property. This is used by sliders, spinners
/// and other widgets that edit a value.
#[derive(Clone, Debug)]
pub struct ValueChange<T>(pub T);

impl<T: Send + Sync + 'static> Event for ValueChange<T> {
    type Traversal = &'static ChildOf;

    const AUTO_PROPAGATE: bool = true;
}

/// An event which is emitted when a button is clicked. This is different from the
/// [`Pointer<Click>`] event, because it's also emitted when the button is focused and the `Enter`
/// or `Space` key is pressed.
#[derive(Clone, Debug)]
pub struct ButtonClicked;

impl Event for ButtonClicked {
    type Traversal = &'static ChildOf;

    const AUTO_PROPAGATE: bool = true;
}
