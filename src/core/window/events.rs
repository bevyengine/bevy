use super::{WindowDescriptor, WindowId};

#[derive(Debug, Clone)]
pub struct WindowResize {
    pub id: WindowId,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct CreateWindow {
    pub descriptor: WindowDescriptor,
}

#[derive(Debug, Clone)]
pub struct WindowCreated {
    pub id: WindowId,
}
