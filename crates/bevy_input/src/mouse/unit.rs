/// A description of how a value of a [`MouseWheel`](crate::mouse::MouseWheel) event has to be interpreted.
///
/// The value of the event can either be interpreted as the amount of lines or the amount of pixels
/// to scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseScrollUnit {
    /// The line scroll unit.
    ///
    /// The delta of the associated [`MouseWheel`](crate::mouse::MouseWheel) event corresponds
    /// to the amount of lines or rows to scroll.
    Line,
    /// The pixel scroll unit.
    ///
    /// The delta of the associated [`MouseWheel`](crate::mouse::MouseWheel) event corresponds
    /// to the amount of pixels to scroll.
    Pixel,
}
