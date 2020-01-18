use crate::render::render_graph_2::{PassDescriptor, PipelineDescriptor};
use std::collections::HashMap;

// holds on to passes, pipeline descriptions, instances
// passes: shadow, forward
pub struct RenderGraph {
    pub pipeline_descriptors: HashMap<String, PipelineDescriptor>,
    pub pass_descriptors: HashMap<String, PassDescriptor>,
    pub pass_pipelines: HashMap<String, Vec<String>>,
}

impl Default for RenderGraph {
    fn default() -> Self {
        RenderGraph {
            pipeline_descriptors: HashMap::new(),
            pass_descriptors: HashMap::new(),
            pass_pipelines: HashMap::new(),
        }
    }
}

impl RenderGraph {
    pub fn build() -> RenderGraphBuilder {
        RenderGraphBuilder {
            render_graph: RenderGraph::default(),
            current_pass: None,
        }
    }
}

pub struct RenderGraphBuilder {
    render_graph: RenderGraph,
    current_pass: Option<String>,
}

impl RenderGraphBuilder {
    pub fn add_pass(mut self, name: &str, pass: PassDescriptor) -> Self {
        self.current_pass = Some(name.to_string());
        self.render_graph
            .pass_descriptors
            .insert(name.to_string(), pass);
        self
    }

    pub fn add_pipeline(mut self, name: &str, pipeline: PipelineDescriptor) -> Self {
        self.render_graph
            .pipeline_descriptors
            .insert(name.to_string(), pipeline);
        
        if let Some(current_pass) = self.current_pass.as_ref() {
            if let None = self.render_graph.pass_pipelines.get(current_pass) {
                self.render_graph.pass_pipelines.insert(current_pass.to_string(), Vec::new());
            }

            let pass_pipelines = self.render_graph.pass_pipelines.get_mut(current_pass).unwrap();
            pass_pipelines.push(name.to_string());
        }

        self
    }

    pub fn build(self) -> RenderGraph {
        self.render_graph
    }
}

/*
RenderGraph::build()
.AddPass("forward", Pass {

})
.AddPipeline(Pipeline::build()
  .with_vertex_shader("pbr.vert")
  .with_fragment_shader("pbr.frag")
  .add_vertex_layout(Vertex::get_layout()) // maybe someday reflect this using spirv-reflect
  .add_uniform_binding("camera_resource", "shader_camera") // if a uniform is not bound directly, and no uniforms are set on entity, produce an error
  .add_texture_binding("some_texture", "shader_texture") // if a uniform is not bound directly, and no uniforms are set on entity, produce an error
  .add_draw_target(MeshDrawTarget)
  .add_draw_target(InstancedMeshDrawTarget)
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
