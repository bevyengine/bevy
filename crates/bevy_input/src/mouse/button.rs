/// A button on a mouse device.
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`Input`](crate::Input) to create a `Bevy`
/// resource. The resource stores the data of the buttons of a mouse and can be accessed
/// inside of a system.
///
/// ## Access
///
/// To access the resource use one of the following:
/// - Non-mutable access of the mouse inputs: `Res<Input<MouseButton>>`
/// - Mutable access of the mouse inputs: `ResMut<Input<MouseButton>>`
///
/// ## Updating
///
/// The resource is updated inside of the [`mouse_button_input_system`](crate::mouse::mouse_button_input_system).
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButton {
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
    /// The middle mouse button.
    Middle,
    /// Another mouse button with the associated number.
    Other(u16),
}
