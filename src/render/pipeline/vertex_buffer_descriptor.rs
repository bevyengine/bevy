use super::VertexFormat;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexBufferDescriptor {
    pub name: String,
    pub stride: u64,
    pub step_mode: InputStepMode,
    pub attributes: Vec<VertexAttributeDescriptor>,
}

impl VertexBufferDescriptor {
    pub fn sync_with_descriptor(&mut self, descriptor: &VertexBufferDescriptor) {
        for attribute in self.attributes.iter_mut() {
            let descriptor_attribute = descriptor
                .attributes
                .iter()
                .find(|a| a.name == attribute.name)
                .unwrap_or_else(|| {
                    panic!(
                        "Encountered unsupported Vertex Buffer Attribute: {}",
                        attribute.name
                    );
                });
            attribute.offset = descriptor_attribute.offset;
        }

        self.stride = descriptor.stride;
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum InputStepMode {
    Vertex = 0,
    Instance = 1,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexAttributeDescriptor {
    pub name: String,
    pub offset: u64,
    pub format: VertexFormat,
    pub shader_location: u32,
}
