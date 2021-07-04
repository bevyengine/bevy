use crate::{
    camera::{ActiveCameras, VisibleEntities},
    draw::{Draw, RenderCommand},
    pass::{ClearColor, LoadOp, PassDescriptor, TextureAttachment},
    pipeline::{IndexFormat, PipelineDescriptor},
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{
        BindGroupId, BufferId, RenderContext, RenderResourceBindings, RenderResourceContext,
        RenderResourceType,
    },
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    query::{QueryState, ReadOnlyFetch, WorldQuery},
    world::{Mut, World},
};
use bevy_utils::{tracing::debug, HashMap};
use std::fmt;

pub struct PassNode<Q: WorldQuery> {
    descriptor: PassDescriptor,
    inputs: Vec<ResourceSlotInfo>,
    cameras: Vec<String>,
    color_attachment_input_indices: Vec<Option<usize>>,
    color_resolve_target_indices: Vec<Option<usize>>,
    depth_stencil_attachment_input_index: Option<usize>,
    default_clear_color_inputs: Vec<usize>,
    query_state: Option<QueryState<Q>>,
    commands: Vec<RenderCommand>,
}

impl<Q: WorldQuery> fmt::Debug for PassNode<Q> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PassNode")
            .field("descriptor", &self.descriptor)
            .field("inputs", &self.inputs)
            .field("cameras", &self.cameras)
            .field(
                "color_attachment_input_indices",
                &self.color_attachment_input_indices,
            )
            .field(
                "color_resolve_target_indices",
                &self.color_resolve_target_indices,
            )
            .field(
                "depth_stencil_attachment_input_index",
                &self.depth_stencil_attachment_input_index,
            )
            .field(
                "default_clear_color_inputs",
                &self.default_clear_color_inputs,
            )
            .finish()
    }
}

impl<Q: WorldQuery> PassNode<Q> {
    pub fn new(descriptor: PassDescriptor) -> Self {
        let mut inputs = Vec::new();
        let mut color_attachment_input_indices = Vec::new();
        let mut color_resolve_target_indices = Vec::new();
        for color_attachment in descriptor.color_attachments.iter() {
            if let TextureAttachment::Input(ref name) = color_attachment.attachment {
                color_attachment_input_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            } else {
                color_attachment_input_indices.push(None);
            }

            if let Some(TextureAttachment::Input(ref name)) = color_attachment.resolve_target {
                color_resolve_target_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            } else {
                color_resolve_target_indices.push(None);
            }
        }

        let mut depth_stencil_attachment_input_index = None;
        if let Some(ref depth_stencil_attachment) = descriptor.depth_stencil_attachment {
            if let TextureAttachment::Input(ref name) = depth_stencil_attachment.attachment {
                depth_stencil_attachment_input_index = Some(inputs.len());
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            }
        }

        PassNode {
            descriptor,
            inputs,
            cameras: Vec::new(),
            color_attachment_input_indices,
            color_resolve_target_indices,
            depth_stencil_attachment_input_index,
            default_clear_color_inputs: Vec::new(),
            query_state: None,
            commands: Vec::new(),
        }
    }

    pub fn add_camera(&mut self, camera_name: &str) {
        self.cameras.push(camera_name.to_string());
    }

    pub fn use_default_clear_color(&mut self, color_attachment_index: usize) {
        self.default_clear_color_inputs.push(color_attachment_index);
    }
}

impl<Q: WorldQuery + Send + Sync + 'static> Node for PassNode<Q>
where
    Q::Fetch: ReadOnlyFetch,
{
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn prepare(&mut self, world: &mut World) {
        let query_state = self.query_state.get_or_insert_with(|| world.query());
        let cameras = &self.cameras;
        let commands = &mut self.commands;
        world.resource_scope(|world, mut active_cameras: Mut<ActiveCameras>| {
            let mut pipeline_camera_commands = HashMap::default();
            let pipelines = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();
            let render_resource_context = &**world
                .get_resource::<Box<dyn RenderResourceContext>>()
                .unwrap();

            for camera_name in cameras.iter() {
                let active_camera = if let Some(active_camera) = active_cameras.get_mut(camera_name)
                {
                    active_camera
                } else {
                    continue;
                };

                let visible_entities = if let Some(entity) = active_camera.entity {
                    world.get::<VisibleEntities>(entity).unwrap()
                } else {
                    continue;
                };
                for visible_entity in visible_entities.iter() {
                    if query_state.get(world, visible_entity.entity).is_err() {
                        // visible entity does not match the Pass query
                        continue;
                    }

                    let draw = if let Some(draw) = world.get::<Draw>(visible_entity.entity) {
                        draw
                    } else {
                        continue;
                    };

                    for render_command in draw.render_commands.iter() {
                        commands.push(render_command.clone());
                        // whenever a new pipeline is set, ensure the relevant camera bind groups
                        // are set
                        if let RenderCommand::SetPipeline { pipeline } = render_command {
                            let bind_groups = pipeline_camera_commands
                                .entry(pipeline.clone_weak())
                                .or_insert_with(|| {
                                    let descriptor = pipelines.get(pipeline).unwrap();
                                    let layout = descriptor.get_layout().unwrap();
                                    let mut commands = Vec::new();
                                    for bind_group_descriptor in layout.bind_groups.iter() {
                                        if let Some(bind_group) =
                                            active_camera.bindings.update_bind_group(
                                                bind_group_descriptor,
                                                render_resource_context,
                                            )
                                        {
                                            commands.push(RenderCommand::SetBindGroup {
                                                index: bind_group_descriptor.index,
                                                bind_group: bind_group.id,
                                                dynamic_uniform_indices: bind_group
                                                    .dynamic_uniform_indices
                                                    .clone(),
                                            })
                                        }
                                    }
                                    commands
                                });

                            commands.extend(bind_groups.iter().cloned());
                        }
                    }
                }
            }
        });
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        for (i, color_attachment) in self.descriptor.color_attachments.iter_mut().enumerate() {
            if self.default_clear_color_inputs.contains(&i) {
                if let Some(default_clear_color) = world.get_resource::<ClearColor>() {
                    color_attachment.ops.load = LoadOp::Clear(default_clear_color.0);
                }
            }
            if let Some(input_index) = self.color_attachment_input_indices[i] {
                color_attachment.attachment =
                    TextureAttachment::Id(input.get(input_index).unwrap().get_texture().unwrap());
            }
            if let Some(input_index) = self.color_resolve_target_indices[i] {
                color_attachment.resolve_target = Some(TextureAttachment::Id(
                    input.get(input_index).unwrap().get_texture().unwrap(),
                ));
            }
        }

        if let Some(input_index) = self.depth_stencil_attachment_input_index {
            self.descriptor
                .depth_stencil_attachment
                .as_mut()
                .unwrap()
                .attachment =
                TextureAttachment::Id(input.get(input_index).unwrap().get_texture().unwrap());
        }

        let render_resource_bindings = world.get_resource::<RenderResourceBindings>().unwrap();
        let pipelines = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();

        let mut draw_state = DrawState::default();
        let commands = &mut self.commands;
        render_context.begin_pass(
            &self.descriptor,
            &render_resource_bindings,
            &mut |render_pass| {
            for render_command in commands.drain(..) {
                match render_command {
                    RenderCommand::SetPipeline { pipeline } => {
                        if draw_state.is_pipeline_set(pipeline.clone_weak()) {
                            continue;
                        }
                        render_pass.set_pipeline(&pipeline);
                        let descriptor = pipelines.get(&pipeline).unwrap();
                        draw_state.set_pipeline(&pipeline, descriptor);
                    }
                    RenderCommand::DrawIndexed {
                        base_vertex,
                        indices,
                        instances,
                    } => {
                        if draw_state.can_draw_indexed() {
                            render_pass.draw_indexed(
                                indices.clone(),
                                base_vertex,
                                instances.clone(),
                            );
                        } else {
                            debug!("Could not draw indexed because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                        }
                    }
                    RenderCommand::Draw { vertices, instances } => {
                        if draw_state.can_draw() {
                            render_pass.draw(vertices.clone(), instances.clone());
                        } else {
                            debug!("Could not draw because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                        }
                    }
                    RenderCommand::SetVertexBuffer {
                        buffer,
                        offset,
                        slot,
                    } => {
                        if draw_state.is_vertex_buffer_set(slot, buffer, offset) {
                            continue;
                        }
                        render_pass.set_vertex_buffer(slot, buffer, offset);
                        draw_state.set_vertex_buffer(slot, buffer, offset);
                    }
                    RenderCommand::SetIndexBuffer { buffer, offset, index_format } => {
                        if draw_state.is_index_buffer_set(buffer, offset, index_format) {
                            continue;
                        }
                        render_pass.set_index_buffer(buffer, offset, index_format);
                        draw_state.set_index_buffer(buffer, offset, index_format);
                    }
                    RenderCommand::SetBindGroup {
                        index,
                        bind_group,
                        dynamic_uniform_indices,
                    } => {
                        if dynamic_uniform_indices.is_none() && draw_state.is_bind_group_set(index, bind_group) {
                            continue;
                        }
                        let pipeline = pipelines.get(draw_state.pipeline.as_ref().unwrap()).unwrap();
                        let layout = pipeline.get_layout().unwrap();
                        let bind_group_descriptor = layout.get_bind_group(index).unwrap();
                        render_pass.set_bind_group(
                            index,
                            bind_group_descriptor.id,
                            bind_group,
                            dynamic_uniform_indices.as_deref()
                        );
                        draw_state.set_bind_group(index, bind_group);
                    }
                }
            }
        });
    }
}

/// Tracks the current pipeline state to ensure draw calls are valid.
#[derive(Debug, Default)]
struct DrawState {
    pipeline: Option<Handle<PipelineDescriptor>>,
    bind_groups: Vec<Option<BindGroupId>>,
    vertex_buffers: Vec<Option<(BufferId, u64)>>,
    index_buffer: Option<(BufferId, u64, IndexFormat)>,
}

impl DrawState {
    pub fn set_bind_group(&mut self, index: u32, bind_group: BindGroupId) {
        self.bind_groups[index as usize] = Some(bind_group);
    }

    pub fn is_bind_group_set(&self, index: u32, bind_group: BindGroupId) -> bool {
        self.bind_groups[index as usize] == Some(bind_group)
    }

    pub fn set_vertex_buffer(&mut self, index: u32, buffer: BufferId, offset: u64) {
        self.vertex_buffers[index as usize] = Some((buffer, offset));
    }

    pub fn is_vertex_buffer_set(&self, index: u32, buffer: BufferId, offset: u64) -> bool {
        self.vertex_buffers[index as usize] == Some((buffer, offset))
    }

    pub fn set_index_buffer(&mut self, buffer: BufferId, offset: u64, index_format: IndexFormat) {
        self.index_buffer = Some((buffer, offset, index_format));
    }

    pub fn is_index_buffer_set(
        &self,
        buffer: BufferId,
        offset: u64,
        index_format: IndexFormat,
    ) -> bool {
        self.index_buffer == Some((buffer, offset, index_format))
    }

    pub fn can_draw(&self) -> bool {
        self.bind_groups.iter().all(|b| b.is_some())
            && self.vertex_buffers.iter().all(|v| v.is_some())
    }

    pub fn can_draw_indexed(&self) -> bool {
        self.can_draw() && self.index_buffer.is_some()
    }

    pub fn is_pipeline_set(&self, pipeline: Handle<PipelineDescriptor>) -> bool {
        self.pipeline == Some(pipeline)
    }

    pub fn set_pipeline(
        &mut self,
        handle: &Handle<PipelineDescriptor>,
        descriptor: &PipelineDescriptor,
    ) {
        self.bind_groups.clear();
        self.vertex_buffers.clear();
        self.index_buffer = None;

        self.pipeline = Some(handle.clone_weak());
        let layout = descriptor.get_layout().unwrap();
        self.bind_groups.resize(layout.bind_groups.len(), None);
        self.vertex_buffers
            .resize(layout.vertex_buffer_descriptors.len(), None);
    }
}
