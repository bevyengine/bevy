use std::{borrow::Cow, mem, num::NonZeroU64, ops::Deref};

use bevy_asset::Handle;
use bevy_math::UVec3;
use bevy_render::render_resource::{
    BindGroup, BindGroupLayout, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType,
    BufferUsages, Sampler, Shader, ShaderDefVal, ShaderStages, StorageTextureAccess,
    TextureDimension, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::core::{
    resource::{
        RenderDependencies, RenderGraphBindGroupDescriptor, RenderGraphBindGroupEntry,
        RenderGraphBindingResource, RenderGraphBufferBinding, RenderGraphComputePipelineDescriptor,
        RenderGraphTextureViewDescriptor, RenderHandle,
    },
    Label, RenderGraphBuilder,
};

pub struct BindGroupBuilder<'a, 'b: 'a, 'g: 'b> {
    label: Label<'g>,
    graph: &'a mut RenderGraphBuilder<'b, 'g>,
    shader_stages: ShaderStages,
    layout: Vec<BindGroupLayoutEntry>,
    entries: Vec<RenderGraphBindGroupEntry<'g>>,
}

impl<'a, 'b: 'a, 'g: 'b> BindGroupBuilder<'a, 'b, 'g> {
    pub fn new(
        graph: &'a mut RenderGraphBuilder<'b, 'g>,
        label: Label<'g>,
        shader_stages: ShaderStages,
    ) -> Self {
        Self {
            label,
            graph,
            shader_stages,
            layout: Vec::new(),
            entries: Vec::new(),
        }
    }

    pub fn internal_graph(&mut self) -> &mut RenderGraphBuilder<'b, 'g> {
        self.graph
    }

    pub fn set_shader_stages(&mut self, shader_stages: ShaderStages) -> &mut Self {
        self.shader_stages = shader_stages;
        self
    }

    pub fn sampler(&mut self, sampler: RenderHandle<'g, Sampler>) -> &mut Self {
        let descriptor = self.graph.meta(sampler);
        self.layout.push(BindGroupLayoutEntry {
            binding: self.layout.len() as u32,
            visibility: self.shader_stages,
            ty: BindingType::Sampler(descriptor.binding_type()),
            count: None,
        });
        self.entries.push(RenderGraphBindGroupEntry {
            binding: self.entries.len() as u32,
            resource: RenderGraphBindingResource::Sampler(sampler),
        });
        self
    }

    pub fn texture(&mut self, texture_view: RenderHandle<'g, TextureView>) -> &mut Self {
        let RenderGraphTextureViewDescriptor {
            texture,
            descriptor,
        } = self.graph.meta(texture_view).clone();
        self.graph
            .add_usages(texture, TextureUsages::TEXTURE_BINDING);
        let features = self.graph.features();
        let texture_descriptor = self.graph.meta(texture);
        self.layout.push(BindGroupLayoutEntry {
            binding: self.layout.len() as u32,
            visibility: self.shader_stages,
            ty: BindingType::Texture {
                sample_type: descriptor
                    .format
                    .unwrap_or(texture_descriptor.format)
                    .sample_type(Some(descriptor.aspect), Some(features))
                    .expect("Unable to determine texture sample type from format"),
                view_dimension: descriptor.dimension.unwrap_or(
                    match texture_descriptor.dimension {
                        TextureDimension::D1 => TextureViewDimension::D1,
                        TextureDimension::D2 => TextureViewDimension::D2,
                        TextureDimension::D3 => TextureViewDimension::D3,
                    },
                ),
                multisampled: texture_descriptor.sample_count != 1,
            },
            count: None,
        });
        self.entries.push(RenderGraphBindGroupEntry {
            binding: self.entries.len() as u32,
            resource: RenderGraphBindingResource::TextureView(texture_view),
        });
        self
    }

    #[inline]
    pub fn read_storage_texture(
        &mut self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> &mut Self {
        self.storage_texture(texture_view, StorageTextureAccess::ReadOnly)
    }

    #[inline]
    pub fn write_storage_texture(
        &mut self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> &mut Self {
        self.storage_texture(texture_view, StorageTextureAccess::WriteOnly)
    }

    #[inline]
    pub fn read_write_storage_texture(
        &mut self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> &mut Self {
        self.storage_texture(texture_view, StorageTextureAccess::ReadWrite)
    }

    pub fn storage_texture(
        &mut self,
        texture_view: RenderHandle<'g, TextureView>,
        access: StorageTextureAccess,
    ) -> &mut Self {
        let RenderGraphTextureViewDescriptor {
            texture,
            descriptor:
                TextureViewDescriptor {
                    format: view_format,
                    dimension: view_dimension,
                    ..
                },
        } = *self.graph.meta(texture_view);
        self.graph
            .add_usages(texture, TextureUsages::STORAGE_BINDING);
        let texture_descriptor = self.graph.meta(texture);
        self.layout.push(BindGroupLayoutEntry {
            binding: self.layout.len() as u32,
            visibility: self.shader_stages,
            ty: BindingType::StorageTexture {
                access,
                format: view_format.unwrap_or(texture_descriptor.format),
                view_dimension: view_dimension.unwrap_or({
                    match texture_descriptor.dimension {
                        TextureDimension::D1 => TextureViewDimension::D1,
                        TextureDimension::D2 => TextureViewDimension::D2,
                        TextureDimension::D3 => TextureViewDimension::D3,
                    }
                }),
            },
            count: None,
        });
        self.entries.push(RenderGraphBindGroupEntry {
            binding: self.entries.len() as u32,
            resource: RenderGraphBindingResource::TextureView(texture_view),
        });
        self
    }

    pub fn uniform_buffer(&mut self, buffer: RenderHandle<'g, Buffer>) -> &mut Self {
        self.graph.add_usages(buffer, BufferUsages::UNIFORM);

        self.layout.push(BindGroupLayoutEntry {
            binding: self.layout.len() as u32,
            visibility: self.shader_stages,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self.entries.push(RenderGraphBindGroupEntry {
            binding: self.entries.len() as u32,
            resource: RenderGraphBindingResource::Buffer(RenderGraphBufferBinding {
                buffer,
                offset: 0,
                size: None,
            }),
        });
        self
    }

    pub fn dynamic_uniform_buffer(
        &mut self,
        buffer: RenderHandle<'g, Buffer>,
        offset: u64,
        binding_size: NonZeroU64,
    ) -> &mut Self {
        self.graph.add_usages(buffer, BufferUsages::UNIFORM);
        self.layout.push(BindGroupLayoutEntry {
            binding: self.layout.len() as u32,
            visibility: self.shader_stages,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(binding_size),
            },
            count: None,
        });
        self.entries.push(RenderGraphBindGroupEntry {
            binding: self.layout.len() as u32,
            resource: RenderGraphBindingResource::Buffer(RenderGraphBufferBinding {
                buffer,
                offset,
                size: Some(binding_size),
            }),
        });
        self
    }

    pub fn read_storage_buffer(&mut self, buffer: RenderHandle<'g, Buffer>) -> &mut Self {
        self.storage_buffer(buffer, true)
    }

    pub fn write_storage_buffer(&mut self, buffer: RenderHandle<'g, Buffer>) -> &mut Self {
        self.storage_buffer(buffer, false)
    }

    pub fn read_write_storage_buffer(&mut self, buffer: RenderHandle<'g, Buffer>) -> &mut Self {
        self.storage_buffer(buffer, false)
    }

    pub fn storage_buffer(
        &mut self,
        buffer: RenderHandle<'g, Buffer>,
        read_only: bool,
    ) -> &mut Self {
        self.graph.add_usages(buffer, BufferUsages::STORAGE);
        self.layout.push(BindGroupLayoutEntry {
            binding: self.layout.len() as u32,
            visibility: self.shader_stages,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self.entries.push(RenderGraphBindGroupEntry {
            binding: self.entries.len() as u32,
            resource: RenderGraphBindingResource::Buffer(RenderGraphBufferBinding {
                buffer,
                offset: 0,
                size: None,
            }),
        });
        self
    }

    pub fn build(&mut self) -> RenderHandle<'g, BindGroup> {
        let layout = self.graph.new_resource(mem::take(&mut self.layout));
        let bind_group = self.graph.new_resource(RenderGraphBindGroupDescriptor {
            label: mem::take(&mut self.label),
            layout,
            entries: mem::take(&mut self.entries),
        });
        bind_group
    }
}

pub struct ComputePass<'a, 'b: 'a, 'g: 'b> {
    label: Label<'static>,
    entry_point: Cow<'static, str>,
    bind_group: BindGroupBuilder<'a, 'b, 'g>,
    shader: Handle<Shader>,
    shader_defs: Vec<ShaderDefVal>,
    dispatch_size: UVec3,
}

impl<'a, 'b: 'a, 'g: 'b> ComputePass<'a, 'b, 'g> {
    pub fn new(
        graph: &'a mut RenderGraphBuilder<'b, 'g>,
        label: Label<'static>,
        shader: Handle<Shader>,
        entry_point: Cow<'static, str>,
    ) -> Self {
        Self {
            label: label.clone(),
            entry_point,
            bind_group: BindGroupBuilder::new(graph, label, ShaderStages::COMPUTE),
            shader,
            dispatch_size: UVec3::ONE,
            shader_defs: Vec::new(),
        }
    }

    pub fn sampler(&mut self, sampler: RenderHandle<'g, Sampler>) -> &mut Self {
        self.bind_group.sampler(sampler);
        self
    }

    pub fn texture(&mut self, texture: RenderHandle<'g, TextureView>) -> &mut Self {
        self.bind_group.texture(texture);
        self
    }

    pub fn read_storage_texture(&mut self, texture: RenderHandle<'g, TextureView>) -> &mut Self {
        self.bind_group.read_storage_texture(texture);
        self
    }

    pub fn write_storage_texture(&mut self, texture: RenderHandle<'g, TextureView>) -> &mut Self {
        self.bind_group.write_storage_texture(texture);
        self
    }

    pub fn define(&mut self, defines: &[ShaderDefVal]) -> &mut Self {
        self.shader_defs.extend_from_slice(defines);
        self
    }

    pub fn dispatch(&mut self, size: UVec3) -> &mut Self {
        self.dispatch_size = size;
        self
    }

    pub fn dispatch_1d(&mut self, x: u32) -> &mut Self {
        self.dispatch(UVec3 { x, y: 1, z: 1 })
    }

    pub fn dispatch_2d(&mut self, x: u32, y: u32) -> &mut Self {
        self.dispatch(UVec3 { x, y, z: 1 })
    }

    pub fn dispatch_3d(&mut self, x: u32, y: u32, z: u32) -> &mut Self {
        self.dispatch(UVec3 { x, y, z })
    }

    pub fn build(&mut self) {
        let bind_group = self.bind_group.build();
        let graph = self.bind_group.internal_graph();
        let pipeline = graph.new_resource(RenderGraphComputePipelineDescriptor {
            label: self.label.clone(),
            layout: vec![graph.meta(bind_group).descriptor.layout],
            push_constant_ranges: Vec::new(),
            shader: mem::take(&mut self.shader),
            shader_defs: mem::take(&mut self.shader_defs),
            entry_point: mem::take(&mut self.entry_point),
        });

        let mut dependencies = RenderDependencies::new();
        dependencies.add_bind_group(graph, bind_group);
        dependencies.read(pipeline);

        let dispatch_size = self.dispatch_size;

        graph.add_compute_node(
            mem::take(&mut self.label),
            dependencies,
            move |ctx, pass| {
                pass.set_bind_group(0, ctx.get(bind_group).deref(), &[]);
                pass.set_pipeline(ctx.get(pipeline).deref());
                pass.dispatch_workgroups(dispatch_size.x, dispatch_size.y, dispatch_size.z);
            },
        );
    }
}
