mod event;
pub use event::*;

/// An event that indicates the app should exit. This will fully exit the app process.
pub struct AppExit;
