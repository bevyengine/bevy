use super::*;
use crate::core::*;

/// A key input event from a keyboard device
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub scan_code: u32,
    pub key_code: Option<KeyCode>,
    pub state: ElementState,
}
