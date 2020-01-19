use crate::render::render_graph_2::{PassDescriptor, PipelineDescriptor};
use std::collections::HashMap;

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