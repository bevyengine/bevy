use uuid::Uuid;

// TODO: Rename to RenderResourceId
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderResource(Uuid);

impl RenderResource {
    pub fn new() -> Self {
        RenderResource(Uuid::new_v4())
    }
}
