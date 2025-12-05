use super::resource_cache::ResourceCache;
use crate::{
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
        ComputePipelineDescriptor, IntoBindGroupLayoutEntryBuilderArray, IntoBindingArray,
    },
    renderer::RenderDevice,
    PipelineCache as PipelineCompiler,
};
use bevy_asset::Handle;
use bevy_shader::{Shader, ShaderDefVal};
use bytemuck::NoUninit;
use std::borrow::Cow;
use wgpu::{BindGroupDescriptor, ComputePass, DynamicOffset, PushConstantRange, ShaderStages};

pub struct ComputeCommandBuilder<'a> {
    pass: &'a mut ComputePass<'static>,
    pass_name: &'a str,
    shader: Handle<Shader>,
    entry_point: Option<&'static str>,
    shader_defs: Vec<ShaderDefVal>,
    push_constants: Option<&'a [u8]>,
    bind_groups: Vec<(Option<BindGroup>, &'a [DynamicOffset])>,
    bind_group_layouts: Vec<BindGroupLayoutDescriptor>,
    resource_cache: &'a mut ResourceCache,
    pipeline_compiler: &'a PipelineCompiler,
    render_device: &'a RenderDevice,
}

impl<'a> ComputeCommandBuilder<'a> {
    pub fn new(
        pass: &'a mut ComputePass<'static>,
        pass_name: &'a str,
        resource_cache: &'a mut ResourceCache,
        pipeline_compiler: &'a PipelineCompiler,
        render_device: &'a RenderDevice,
    ) -> Self {
        Self {
            pass,
            pass_name,
            shader: Handle::default(),
            entry_point: None,
            shader_defs: Vec::new(),
            push_constants: None,
            bind_groups: Vec::new(),
            bind_group_layouts: Vec::new(),
            resource_cache,
            pipeline_compiler,
            render_device,
        }
    }

    pub fn shader(mut self, shader: Handle<Shader>) -> Self {
        self.shader = shader;
        self
    }

    pub fn entry_point(mut self, entry_point: &'static str) -> Self {
        self.entry_point = Some(entry_point);
        self
    }

    pub fn shader_def(mut self, shader_def: impl Into<ShaderDefVal>) -> Self {
        self.shader_defs.push(shader_def.into());
        self
    }

    pub fn shader_def_if(mut self, shader_def: impl Into<ShaderDefVal>, condition: bool) -> Self {
        if condition {
            self.shader_defs.push(shader_def.into());
        }
        self
    }

    pub fn push_constants<T: NoUninit>(mut self, push_constants: &'a [T]) -> Self {
        self.push_constants = Some(bytemuck::cast_slice(push_constants));
        self
    }

    pub fn bind_resources<'b, const N: usize>(
        self,
        resources: impl IntoBindingArray<'b, N> + IntoBindGroupLayoutEntryBuilderArray<N> + Clone,
    ) -> Self {
        self.bind_resources_with_dynamic_offsets((resources, &[]))
    }

    pub fn bind_resources_with_dynamic_offsets<'b, const N: usize>(
        mut self,
        (resources, dynamic_offsets): (
            impl IntoBindingArray<'b, N> + IntoBindGroupLayoutEntryBuilderArray<N> + Clone,
            &'a [DynamicOffset],
        ),
    ) -> Self {
        let layout_descriptor = BindGroupLayoutDescriptor::new(
            self.pass_name.to_owned(),
            &BindGroupLayoutEntries::sequential(ShaderStages::COMPUTE, resources.clone()),
        );

        let descriptor = BindGroupDescriptor {
            label: Some(self.pass_name),
            layout: &self
                .pipeline_compiler
                .get_bind_group_layout(&layout_descriptor),
            entries: &BindGroupEntries::sequential(resources),
        };

        // TODO
        // self.bind_groups.push(Some(
        //     self.resource_cache
        //         .get_or_create_bind_group(descriptor, self.render_device),
        // ));
        self.bind_groups.push((
            Some(
                self.render_device
                    .wgpu_device()
                    .create_bind_group(&descriptor)
                    .into(),
            ),
            dynamic_offsets,
        ));

        self.bind_group_layouts.push(layout_descriptor);

        self
    }

    pub fn bind_group(
        mut self,
        bind_group: impl Into<Option<BindGroup>>,
        layout: BindGroupLayoutDescriptor,
    ) -> Self {
        self.bind_groups.push((bind_group.into(), &[]));
        self.bind_group_layouts.push(layout);
        self
    }

    pub fn bind_group_with_dynamic_offsets(
        mut self,
        bind_group: BindGroup,
        dynamic_offsets: &'a [DynamicOffset],
        layout: BindGroupLayoutDescriptor,
    ) -> Self {
        self.bind_groups.push((Some(bind_group), dynamic_offsets));
        self.bind_group_layouts.push(layout);
        self
    }

    #[must_use]
    pub fn dispatch_1d(mut self, x: u32) -> Option<Self> {
        self.setup_state()?;
        self.pass.dispatch_workgroups(x, 1, 1);
        Some(self)
    }

    #[must_use]
    pub fn dispatch_2d(mut self, x: u32, y: u32) -> Option<Self> {
        self.setup_state()?;
        self.pass.dispatch_workgroups(x, y, 1);
        Some(self)
    }

    #[must_use]
    pub fn dispatch_3d(mut self, x: u32, y: u32, z: u32) -> Option<Self> {
        self.setup_state()?;
        self.pass.dispatch_workgroups(x, y, z);
        Some(self)
    }

    #[must_use]
    pub fn dispatch_indirect(mut self, buffer: &Buffer) -> Option<Self> {
        self.setup_state()?;
        self.pass.dispatch_workgroups_indirect(buffer, 0);
        Some(self)
    }

    #[must_use]
    fn setup_state(&mut self) -> Option<()> {
        let push_constant_ranges = self
            .push_constants
            .map(|pc| {
                vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..(pc.len() as u32),
                }]
            })
            .unwrap_or_default();

        let pipeline = self.resource_cache.get_or_compile_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some(self.pass_name.to_owned().into()),
                layout: self.bind_group_layouts.clone(),
                push_constant_ranges,
                shader: self.shader.clone(),
                shader_defs: self.shader_defs.clone(),
                entry_point: self.entry_point.map(Cow::from),
                zero_initialize_workgroup_memory: false,
            },
            self.pipeline_compiler,
        )?;

        self.pass.set_pipeline(&pipeline); // TODO: Only set if changed

        if let Some(push_constants) = self.push_constants {
            self.pass.set_push_constants(0, push_constants); // TODO: Only set if pipeline changed
        }

        for (i, (bind_group, dynamic_offsets)) in self.bind_groups.iter().enumerate() {
            // TODO: Only set if changed
            self.pass
                .set_bind_group(i as u32, bind_group.as_deref(), dynamic_offsets);
        }

        Some(())
    }
}
