use crate::WindowId;

pub enum WindowExitMethod {
    /// Exits the app when the last window is closed
    LastClosed,
    /// Exits the app when the primary window is closed
    PrimaryClosed,
    /// Exits the app when any window is closed
    AnyClosed,
    /// Exits the app when a specified window is closed
    WindowClosed(WindowId),
    /// Does not exit the app when windows are closed
    KeepOpen,
}