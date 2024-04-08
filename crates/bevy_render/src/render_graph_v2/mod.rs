mod build;
//mod compute_pass;
pub mod configurator;
pub mod pipeline;
pub mod resource;
pub mod texture;

use self::resource::{
    AsRenderBindGroup, IntoRenderResource, IntoRenderResourceIds, RenderBindGroup, RenderHandle,
    RenderResource,
};
use crate::{
    render_resource::PipelineCache,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryEntityError, QueryFilter, QueryState, ROQueryItem},
    system::{Res, ResMut, Resource},
    world::{EntityRef, World},
};

// Roadmap:
// 1. Autobuild (and cache) bind group layouts, textures, bind groups, and compute pipelines
// 2. Run the graph in the correct order (figure out how the API should handle command encoders/buffers)
// 3. Buffer and sampler support
// 4. Allow importing external textures
// 5. Temporal resources
// 6. Start porting the engine as a proof of concept/demo, and fill in missing features (e.g. raster nodes)
// 7. Auto-insert CPU profiling, GPU profiling, and GPU debug markers (probably need some concept of a group of render nodes)
// 8. Documentation, write an example, and cleanup

#[derive(Resource, Default)]
pub struct RenderGraph {
    // TODO: maybe use a Vec for resource_descriptors, and replace next_id with resource_descriptors.len()
    next_id: u16, //resource_descriptors: HashMap<RenderGraphResourceId, TextureDescriptor<'static>>,
                  // nodes: Vec<RenderGraphNode>,
                  //
                  // bind_group_layouts: HashMap<Box<[BindGroupLayoutEntry]>, BindGroupLayout>,
                  // resources: HashMap<RenderGraphResourceId, Texture>,
                  // pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
}

impl RenderGraph {
    // pub fn create_resource(
    //     &mut self,
    //     descriptor: TextureDescriptor<'static>,
    // ) -> RenderGraphResource {
    //     let id = self.next_id;
    //     self.next_id += 1;
    //
    //     self.resource_descriptors.insert(id, descriptor);
    //
    //     RenderGraphResource { id, generation: 0 }
    // }
    //
    // pub fn add_node(&mut self, node: impl Into<RenderGraphNode>) {
    //     self.nodes.push(node.into());
    // }

    pub(crate) fn run(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // TODO
    }

    pub(crate) fn reset(&mut self) {
        // self.next_id = 0;
        // self.resource_descriptors.clear();
        // self.nodes.clear();
        //
        // TODO: Remove unused resources
    }
}

pub fn run_render_graph(
    mut render_graph: ResMut<RenderGraph>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
    pipeline_cache: Res<PipelineCache>,
) {
    render_graph.reset();
    render_graph.build(render_device, &pipeline_cache);
    render_graph.run(render_device, render_queue);
}

pub struct RenderGraphBuilder<'a> {
    graph: &'a mut RenderGraph,
    world: &'a World,
    view_entity: Entity,
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn new_resource<R: IntoRenderResource>(&mut self, desc: R) -> RenderHandle<R::Resource> {
        todo!()
    }

    pub fn new_bind_group<B: AsRenderBindGroup>(
        &mut self,
        label: Option<&'static str>,
        desc: B,
    ) -> RenderBindGroup {
        todo!()
    }

    pub fn add_node<
        const N: usize,
        F: FnOnce(NodeContext, &RenderDevice, &RenderQueue) + 'static,
    >(
        &mut self,
        dependencies: impl IntoRenderResourceIds<N>,
        node: F,
    ) -> &mut Self {
        self
    }
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn world_query<Q: QueryData>(&self) -> QueryState<Q> {
        self.world.query::<Q>()
    }

    pub fn world_query_filtered<Q: QueryData, F: QueryFilter>(&self) -> QueryState<Q, F> {
        self.world.query_filtered::<Q, F>()
    }

    pub fn view_query<Q: QueryData>(&self) -> Result<ROQueryItem<Q>, QueryEntityError> {
        self.world.query::<Q>().get(&self.world, self.view_entity)
    }

    pub fn view_query_filtered<Q: QueryData, F: QueryFilter>(
        &self,
    ) -> Result<ROQueryItem<Q>, QueryEntityError> {
        self.world
            .query_filtered::<Q, F>()
            .get(&self.world, self.view_entity)
    }

    pub fn resource<R: Resource>(&self) -> &'a R {
        self.world.resource()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&'a R> {
        self.world.get_resource()
    }
}

pub struct NodeContext<'a> {
    graph: &'a RenderGraph,
    world: &'a World,
    view_entity: Entity,
    resource_dependencies: (),
    bind_group_dependencies: (),
}

impl<'a> NodeContext<'a> {
    pub fn world_query<Q: QueryData>(&self) -> QueryState<Q> {
        self.world.query::<Q>()
    }

    pub fn world_query_filtered<Q: QueryData, F: QueryFilter>(&self) -> QueryState<Q, F> {
        self.world.query_filtered::<Q, F>()
    }

    pub fn view_query<Q: QueryData>(&self) -> Result<ROQueryItem<Q>, QueryEntityError> {
        self.world.query::<Q>().get(&self.world, self.view_entity)
    }

    pub fn view_query_filtered<Q: QueryData, F: QueryFilter>(
        &self,
    ) -> Result<ROQueryItem<Q>, QueryEntityError> {
        self.world
            .query_filtered::<Q, F>()
            .get(&self.world, self.view_entity)
    }

    pub fn get<R: RenderResource>(&self, handle: &RenderHandle<R>) -> &'a R {
        todo!()
    }

    pub fn resource<R: Resource>(&self) -> &'a R {
        self.world.resource()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&'a R> {
        self.world.get_resource()
    }
}
