mod pipeline;

pub use pipeline::*;

use crate::prelude::*;
use crate::{asset::Texture, legion::{prelude::{Entity, World}, borrow::{Ref, RefMap}}, render::Albedo};
use std::collections::HashMap;

pub enum ShaderValue<'a> {
    Int(u32),
    Float(f32),
    Vec4(Vec4),
    Texture(&'a Handle<Texture>),
}

type ShaderMaterialSelector = fn(Entity, &World) -> Option<RefMap<&dyn ShaderMaterial>>;
pub struct ShaderMaterials {
    // used for distinguishing 
    pub materials: Vec<ShaderMaterialSelector>
}

impl<'a> ShaderMaterials {
  pub fn new() -> Self {
    ShaderMaterials {
      materials: Vec::new(),
    }
  }

  pub fn add(&mut self, selector: ShaderMaterialSelector) {
    self.materials.push(selector);
  }
}

pub trait ShaderMaterial {
  fn iter_properties(&self) -> std::slice::Iter<&'static str> ;
  fn get_property(&self, name: &str) -> Option<ShaderValue>;
  fn get_selector(&self) -> ShaderMaterialSelector;
}

pub struct StandardMaterial {
  pub albedo: Albedo
}

// create this from a derive macro
const STANDARD_MATERIAL_PROPERTIES: &[&str] = &["albedo"];
impl ShaderMaterial for StandardMaterial {
    fn iter_properties(&self) -> std::slice::Iter<&'static str>  {
      STANDARD_MATERIAL_PROPERTIES.iter()
    }
    fn get_property(&self, name: &str) -> Option<ShaderValue> {
      match name {
        "albedo" => Some(match self.albedo {
          Albedo::Color(color) => ShaderValue::Vec4(color),
          Albedo::Texture(ref texture) => ShaderValue::Texture(texture)
        }),
        _ => None,
      }
    }
    fn get_selector(&self) -> ShaderMaterialSelector {
      |entity, world| { 
        world.get_component::<Self>(entity).map(
          |c: Ref<StandardMaterial>| {
            c.map_into(|s| {
              s as &dyn ShaderMaterial
            })
          }
        )
      }
    }
}


// a named graphics resource provided by a resource provider
struct Resource {
  resource_type: ResourceType,
  name: String,
}

// a resource type
enum ResourceType {
  Texture,
  Uniform,
  Sampler,
}

// impl Into<ShaderMaterial> for StandardMaterial

// manages a specific named resource in a RenderGraph. [my_texture_name: TextureView]-->
// updates resources based on events like "resize" or "update"
// if there are no resources in use, dont run allocate resource provider resources on gpu
trait ResourceProvider {
  fn get_resources(&self) -> &[Resource];
}

// holds on to passes, pipeline descriptions, instances
// passes: shadow, forward
struct RenderGraph {
  pipeline_definitions: HashMap<String, PipelineDefinition>,
  pipeline_instances: HashMap<String, wgpu::RenderPipeline>,
}

struct RenderGraphBuilder {

}

/*
RenderGraph::build()
.AddPass("forward", Pass {

})
.AddPipeline(Pipeline::build()
  .with_vertex_shader("pbr.vert")
  .with_fragment_shader("pbr.frag")
  .with_vertex_layout(Vertex::get_layout()) // maybe someday reflect this using spirv-reflect
  .with_uniform_binding("camera_resource", "shader_camera") // if a uniform is not bound directly, and no uniforms are set on entity, produce an error
  .with_texture_binding("some_texture", "shader_texture") // if a uniform is not bound directly, and no uniforms are set on entity, produce an error
  .with_draw_target(MeshDrawTarget)
  .with_draw_target(InstancedMeshDrawTarget)
)
.AddPipeline(Pipeline::build()
  .with_vertex_shader("ui.vert")
  .with_fragment_shader("ui.frag")
  .with_vertex_layout(Vertex::get_layout())
  .with_draw_target(UiDrawTarget)
)
.AddPass("shadow", Pass {
    render_target: Null
    depth_target: DepthTexture (TextureView)
})
.AddPipeline(Pipeline::build()
  .with_vertex_shader("pbr.vert")
  .with_fragment_shader("pbr.frag")
  .with_vertex_layout(Vertex::get_layout())
  .with_draw_target(ShadowedMeshDrawTarget)
  .with_draw_target(ShadowedInstancedMeshDrawTarget)
)
*/

pub struct RenderPassColorAttachmentDescription {
  /// The actual color attachment.
  pub attachment: String,

  /// The resolve target for this color attachment, if any.
  pub resolve_target: Option<String>,

  /// The beginning-of-pass load operation for this color attachment.
  pub load_op: wgpu::LoadOp,

  /// The end-of-pass store operation for this color attachment.
  pub store_op: wgpu::StoreOp,

  /// The color that will be assigned to every pixel of this attachment when cleared.
  pub clear_color: wgpu::Color,
}

pub struct RenderPassDepthStencilAttachmentDescription {
    pub attachment: String,
    pub depth_load_op: wgpu::LoadOp,
    pub depth_store_op: wgpu::StoreOp,
    pub clear_depth: f32,
    pub stencil_load_op: wgpu::LoadOp,
    pub stencil_store_op: wgpu::StoreOp,
    pub clear_stencil: u32,
}

// A set of pipeline bindings and draw calls with color and depth outputs
struct Pass {
  color_attachments: Vec<RenderPassColorAttachmentDescription>,
  depth_stencil_attachment: Option<RenderPassDepthStencilAttachmentDescription>,
}