/// A button on a mouse device
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButtonCode {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// Unit of scroll
#[derive(Debug, Clone, Copy)]
pub enum MouseScrollUnitCode {
    Line,
    Pixel,
}
