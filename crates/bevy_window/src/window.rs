use uuid::Uuid;

#[derive(Debug)]
pub enum WindowReference {
    Primary,
    Id(WindowId),
}

impl Default for WindowReference {
    fn default() -> Self {
        WindowReference::Primary
    }
    
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(Uuid);

impl WindowId {
    pub fn new() -> Self {
        WindowId(Uuid::new_v4())
    }

    pub fn to_string(&self) -> String {
        self.0.to_simple().to_string()
    }
}

#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
}

impl Window {
    pub fn new(id: WindowId, window_descriptor: &WindowDescriptor) -> Self {
        Window {
            id,
            height: window_descriptor.height,
            width: window_descriptor.width,
            title: window_descriptor.title.clone(),
            vsync: window_descriptor.vsync,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "bevy".to_string(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}
