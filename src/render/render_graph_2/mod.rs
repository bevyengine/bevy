mod pipeline;
mod pass;
mod renderer;
mod shader;
mod render_graph;
mod example;


pub use pipeline::*;
pub use pass::*;
pub use renderer::*;
pub use shader::*;
pub use render_graph::*;

// a named graphics resource provided by a resource provider
pub struct Resource {
  resource_type: ResourceType,
  name: String,
}

// a resource type
enum ResourceType {
  Texture,
  Buffer,
  Sampler,
}

// impl Into<ShaderMaterial> for StandardMaterial

// manages a specific named resource in a RenderGraph. [my_texture_name: TextureView]-->
// updates resources based on events like "resize" or "update"
// if there are no resources in use, dont run allocate resource provider resources on gpu
trait ResourceProvider {
  fn get_resources(&self) -> &[Resource];
}