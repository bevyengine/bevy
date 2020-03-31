use super::{WindowDescriptor, WindowId};

#[derive(Debug, Clone)]
pub struct WindowResized {
    pub id: WindowId,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

#[derive(Debug, Clone)]
pub struct CreateWindow {
    pub descriptor: WindowDescriptor,
}

#[derive(Debug, Clone)]
pub struct WindowCreated {
    pub id: WindowId,
}
