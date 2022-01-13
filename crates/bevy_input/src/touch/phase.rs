/// A phase of a [`TouchInput`](crate::touch::TouchInput).
///
/// ## Usage
///
/// It is used to describe the phase of the touch input that is currently active.
/// This includes a phase that indicates that a touch input has started or ended,
/// or that a finger has moved. There is also a cancelled phase that indicates that
/// the system cancelled the tracking of the finger.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum TouchPhase {
    /// A finger started to touch the touchscreen.
    Started,
    /// A finger moved over the touchscreen.
    Moved,
    /// A finger stopped touching the touchscreen.
    Ended,
    /// The system cancelled the tracking of the finger.
    /// This occurs when the window loses focus, or on iOS if the user moves the
    /// device against their face.
    Cancelled,
}
