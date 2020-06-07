use uuid::Uuid;

// TODO: Rename to RenderResourceId
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderResourceId(Uuid);

impl RenderResourceId {
    pub fn new() -> Self {
        RenderResourceId(Uuid::new_v4())
    }
}
