use uuid::Uuid;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureId(Uuid);

impl TextureId {
    pub fn new() -> Self {
        TextureId(Uuid::new_v4())
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SamplerId(Uuid);

impl SamplerId {
    pub fn new() -> Self {
        SamplerId(Uuid::new_v4())
    }
}
