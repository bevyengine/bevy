use bevy_render::render_resource::{
    BindGroup, BindGroupLayoutEntry, BindingType, Buffer, Sampler, ShaderStages, TextureView,
};

use crate::core::{
    resource::{
        bind_group::{
            RenderGraphBindGroupDescriptor, RenderGraphBindGroupEntry, RenderGraphBindingResource,
        },
        RenderDependencies, RenderHandle,
    },
    Label, RenderGraphBuilder,
};

pub struct BindGroupBuilder<'b, 'g: 'b> {
    graph: &'b mut RenderGraphBuilder<'g>,
    label: Label<'g>,
    shader_stages: ShaderStages,
    layout: Vec<BindGroupLayoutEntry>,
    bindings: Vec<RenderGraphBindGroupEntry<'g>>,
    dependencies: RenderDependencies<'g>,
}

impl<'b, 'g: 'b> BindGroupBuilder<'b, 'g> {
    pub fn new(
        graph: &'b mut RenderGraphBuilder<'g>,
        label: Label<'g>,
        shader_stages: ShaderStages,
    ) -> Self {
        Self {
            graph,
            label,
            shader_stages,
            layout: Vec::new(),
            bindings: Vec::new(),
            dependencies: RenderDependencies::new(),
        }
    }

    pub fn set_shader_stages(&mut self, shader_stages: ShaderStages) -> &mut Self {
        self
    }

    pub fn sampler(&mut self, sampler: RenderHandle<'g, Sampler>) -> &mut Self {
        let descriptor = self.graph.descriptor(sampler);
        self.layout.push(BindGroupLayoutEntry {
            binding: self.layout.len() as u32,
            visibility: self.shader_stages,
            ty: BindingType::Sampler(descriptor.binding_type()),
            count: None,
        });
        self.bindings.push(RenderGraphBindGroupEntry {
            binding: self.bindings.len() as u32,
            resource: RenderGraphBindingResource::Sampler(sampler),
        });
        self.dependencies.read(sampler);
        self
    }

    pub fn texture(&mut self, texture: RenderHandle<'g, TextureView>) -> &mut Self {
        self
    }

    pub fn read_storage_texture(&mut self, texture: RenderHandle<'g, TextureView>) -> &mut Self {
        self
    }

    pub fn write_storage_texture(&mut self, texture: RenderHandle<'g, TextureView>) -> &mut Self {
        // self.graph
        //     .add_usages(texture, TextureUsages::STORAGE_BINDING);
        // let descriptor = self.graph.descriptor_of(texture);
        // self.layout.push(BindGroupLayoutEntry {
        //     binding: self.layout.len(),
        //     visibility: self.shader_stages,
        //     ty: BindingType::StorageTexture {
        //         access: StorageTextureAccess::ReadWrite,
        //         format: (),
        //         view_dimension: (),
        //     },
        //     count: todo!(),
        // });
        self
    }

    pub fn read_buffer(&mut self, buffer: RenderHandle<'g, Buffer>) -> &mut Self {
        self
    }

    pub fn write_buffer(&mut self, buffer: RenderHandle<'g, Buffer>) -> &mut Self {
        self
    }

    pub fn build(self) -> RenderHandle<'g, BindGroup> {
        let layout = self.graph.new_resource(self.layout);
        self.graph.new_resource(RenderGraphBindGroupDescriptor {
            label: self.label,
            layout,
            dependencies: self.dependencies,
            bindings: self.bindings,
        })
    }
}
