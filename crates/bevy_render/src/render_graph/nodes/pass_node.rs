use crate::{
    camera::{ActiveCameras, VisibleEntities},
    draw::{Draw, RenderCommand},
    pass::{ClearColor, LoadOp, PassDescriptor, TextureAttachment},
    pipeline::{
        BindGroupDescriptor, BindType, BindingDescriptor, BindingShaderStage, PipelineDescriptor,
        UniformProperty,
    },
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{
        BindGroup, BindGroupId, BufferId, RenderContext, RenderResourceBindings, RenderResourceType,
    },
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{HecsQuery, ReadOnlyFetch, Resources, World};
use std::{fmt, marker::PhantomData, ops::Deref};

#[derive(Debug)]
struct CameraInfo {
    name: String,
    bind_group_id: Option<BindGroupId>,
}

pub struct PassNode<Q: HecsQuery> {
    descriptor: PassDescriptor,
    inputs: Vec<ResourceSlotInfo>,
    cameras: Vec<CameraInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    color_resolve_target_indices: Vec<Option<usize>>,
    depth_stencil_attachment_input_index: Option<usize>,
    default_clear_color_inputs: Vec<usize>,
    camera_bind_group_descriptor: BindGroupDescriptor,
    _marker: PhantomData<Q>,
}

impl<Q: HecsQuery> fmt::Debug for PassNode<Q> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PassNose")
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
            .field(
                "camera_bind_group_descriptor",
                &self.camera_bind_group_descriptor,
            )
            .finish()
    }
}

impl<Q: HecsQuery> PassNode<Q> {
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

        let camera_bind_group_descriptor = BindGroupDescriptor::new(
            0,
            vec![BindingDescriptor {
                name: "Camera".to_string(),
                index: 0,
                bind_type: BindType::Uniform {
                    dynamic: false,
                    property: UniformProperty::Struct(vec![UniformProperty::Mat4]),
                },
                shader_stage: BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT,
            }],
        );

        PassNode {
            descriptor,
            inputs,
            cameras: Vec::new(),
            color_attachment_input_indices,
            color_resolve_target_indices,
            depth_stencil_attachment_input_index,
            default_clear_color_inputs: Vec::new(),
            camera_bind_group_descriptor,
            _marker: PhantomData::default(),
        }
    }

    pub fn add_camera(&mut self, camera_name: &str) {
        self.cameras.push(CameraInfo {
            name: camera_name.to_string(),
            bind_group_id: None,
        });
    }

    pub fn use_default_clear_color(&mut self, color_attachment_index: usize) {
        self.default_clear_color_inputs.push(color_attachment_index);
    }
}

impl<Q: HecsQuery + Send + Sync + 'static> Node for PassNode<Q>
where
    Q::Fetch: ReadOnlyFetch,
{
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn update(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let render_resource_bindings = resources.get::<RenderResourceBindings>().unwrap();
        let pipelines = resources.get::<Assets<PipelineDescriptor>>().unwrap();
        let active_cameras = resources.get::<ActiveCameras>().unwrap();

        for (i, color_attachment) in self.descriptor.color_attachments.iter_mut().enumerate() {
            if self.default_clear_color_inputs.contains(&i) {
                if let Some(default_clear_color) = resources.get::<ClearColor>() {
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
        for camera_info in self.cameras.iter_mut() {
            let camera_binding =
                if let Some(camera_binding) = render_resource_bindings.get(&camera_info.name) {
                    camera_binding.clone()
                } else {
                    continue;
                };
            if render_context
                .resources()
                .bind_group_descriptor_exists(self.camera_bind_group_descriptor.id)
            {
                let camera_bind_group = BindGroup::build().add_binding(0, camera_binding).finish();
                render_context
                    .resources()
                    .create_bind_group(self.camera_bind_group_descriptor.id, &camera_bind_group);
                camera_info.bind_group_id = Some(camera_bind_group.id);
            }
        }

        render_context.begin_pass(
            &self.descriptor,
            &render_resource_bindings,
            &mut |render_pass| {
                for camera_info in self.cameras.iter() {
                    let camera_bind_group_id= if let Some(bind_group_id) = camera_info.bind_group_id {
                        bind_group_id
                    } else {
                        continue;
                    };

                    // get an ordered list of entities visible to the camera
                    let visible_entities = if let Some(camera_entity) = active_cameras.get(&camera_info.name) {
                        world.get::<VisibleEntities>(camera_entity).unwrap()
                    } else {
                        continue;
                    };

                    // attempt to draw each visible entity
                    let mut draw_state = DrawState::default();
                    for visible_entity in visible_entities.iter() {
                        if let Ok(query_one) = world.query_one::<Q>(visible_entity.entity) {
                            if query_one.get().is_none() {
                                // visible entity does not match the Pass query
                                continue;
                            }
                        }

                        let draw = if let Ok(draw) = world.get::<Draw>(visible_entity.entity) {
                            draw
                        } else {
                            continue;
                        };

                        if !draw.is_visible {
                            continue;
                        }

                        // each Draw component contains an ordered list of render commands. we turn those into actual render commands here
                        for render_command in draw.render_commands.iter() {
                            match render_command {
                                RenderCommand::SetPipeline { pipeline } => {
                                    // TODO: Filter pipelines
                                    render_pass.set_pipeline(*pipeline);
                                    let descriptor = pipelines.get(pipeline).unwrap();
                                    draw_state.set_pipeline(*pipeline, descriptor);

                                    // try to set current camera bind group
                                    let layout = descriptor.get_layout().unwrap();
                                    if let Some(descriptor) = layout.get_bind_group(0) {
                                        if *descriptor == self.camera_bind_group_descriptor {
                                            draw_state.set_bind_group(0, camera_bind_group_id);
                                            render_pass.set_bind_group(
                                                0,
                                                descriptor.id,
                                                camera_bind_group_id,
                                                None
                                            );
                                        }
                                    }
                                }
                                RenderCommand::DrawIndexed {
                                    base_vertex,
                                    indices,
                                    instances,
                                } => {
                                    if draw_state.can_draw_indexed() {
                                        render_pass.draw_indexed(
                                            indices.clone(),
                                            *base_vertex,
                                            instances.clone(),
                                        );
                                    } else {
                                        log::info!("Could not draw indexed because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                                    }
                                }
                                RenderCommand::Draw { vertices, instances } => {
                                    if draw_state.can_draw() {
                                        render_pass.draw(vertices.clone(), instances.clone());
                                    } else {
                                        log::info!("Could not draw because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                                    }
                                }
                                RenderCommand::SetVertexBuffer {
                                    buffer,
                                    offset,
                                    slot,
                                } => {
                                    render_pass.set_vertex_buffer(*slot, *buffer, *offset);
                                    draw_state.set_vertex_buffer(*slot, *buffer);
                                }
                                RenderCommand::SetIndexBuffer { buffer, offset } => {
                                    render_pass.set_index_buffer(*buffer, *offset);
                                    draw_state.set_index_buffer(*buffer)
                                }
                                RenderCommand::SetBindGroup {
                                    index,
                                    bind_group,
                                    dynamic_uniform_indices,
                                } => {
                                    let pipeline = pipelines.get(&draw_state.pipeline.unwrap()).unwrap();
                                    let layout = pipeline.get_layout().unwrap();
                                    let bind_group_descriptor = layout.get_bind_group(*index).unwrap();
                                    render_pass.set_bind_group(
                                        *index,
                                        bind_group_descriptor.id,
                                        *bind_group,
                                        dynamic_uniform_indices
                                            .as_ref()
                                            .map(|indices| indices.deref()),
                                    );
                                    draw_state.set_bind_group(*index, *bind_group);
                                }
                            }
                        }
                    }
                }
            },
        );
    }
}

/// Tracks the current pipeline state to ensure draw calls are valid.
#[derive(Debug, Default)]
struct DrawState {
    pipeline: Option<Handle<PipelineDescriptor>>,
    bind_groups: Vec<Option<BindGroupId>>,
    vertex_buffers: Vec<Option<BufferId>>,
    index_buffer: Option<BufferId>,
}

impl DrawState {
    pub fn set_bind_group(&mut self, index: u32, bind_group: BindGroupId) {
        self.bind_groups[index as usize] = Some(bind_group);
    }

    pub fn set_vertex_buffer(&mut self, index: u32, buffer: BufferId) {
        self.vertex_buffers[index as usize] = Some(buffer);
    }

    pub fn set_index_buffer(&mut self, buffer: BufferId) {
        self.index_buffer = Some(buffer);
    }

    pub fn can_draw(&self) -> bool {
        self.bind_groups.iter().all(|b| b.is_some())
            && self.vertex_buffers.iter().all(|v| v.is_some())
    }

    pub fn can_draw_indexed(&self) -> bool {
        self.can_draw() && self.index_buffer.is_some()
    }

    pub fn set_pipeline(
        &mut self,
        handle: Handle<PipelineDescriptor>,
        descriptor: &PipelineDescriptor,
    ) {
        self.bind_groups.clear();
        self.vertex_buffers.clear();
        self.index_buffer = None;

        self.pipeline = Some(handle);
        let layout = descriptor.get_layout().unwrap();
        self.bind_groups.resize(layout.bind_groups.len(), None);
        self.vertex_buffers
            .resize(layout.vertex_buffer_descriptors.len(), None);
    }
}
