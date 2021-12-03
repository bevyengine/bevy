use crate::WindowId;

#[derive(Debug, Clone)]
pub enum WindowExitMethod {
    /// Exits the app when any window is closed
    AnyClosed,
    /// Exits the app when the primary window is closed
    PrimaryClosed,
    /// Exits the app when the last window is closed
    LastClosed,
    /// Exits the app when a specified window is closed
    WindowClosed(WindowId),
    /// Does not exit the app when windows are closed
    KeepOpen,
}

pub struct WindowsConfig {
    pub exit_method: WindowExitMethod,
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            exit_method: WindowExitMethod::PrimaryClosed,
        }
    }
}
