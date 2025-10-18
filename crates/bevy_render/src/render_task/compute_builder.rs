use bevy_asset::Handle;
use bevy_shader::{Shader, ShaderDefVal};
use bytemuck::NoUninit;
use wgpu::{BindGroup, Buffer, ComputePass};

pub struct ComputeCommandBuilder<'a> {
    compute_pass: &'a mut ComputePass<'static>,
    pass_name: &'a str,
    shader: Handle<Shader>,
    shader_defs: Vec<ShaderDefVal>,
    push_constants: Option<&'a [u8]>,
    bind_groups: Vec<Option<BindGroup>>,
}

impl<'a> ComputeCommandBuilder<'a> {
    pub fn new(compute_pass: &'a mut ComputePass<'static>, pass_name: &'a str) -> Self {
        Self {
            compute_pass,
            pass_name,
            shader: Handle::default(),
            shader_defs: Vec::new(),
            push_constants: None,
            bind_groups: Vec::new(),
        }
    }

    pub fn shader(mut self, shader: Handle<Shader>) -> Self {
        self.shader = shader;
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

    pub fn bind_group<T: NoUninit>(mut self, bind_group: impl Into<Option<BindGroup>>) -> Self {
        self.bind_groups.push(bind_group.into());
        self
    }

    pub fn dispatch_1d(mut self, x: u32) -> Self {
        self.setup_state();
        self.compute_pass.dispatch_workgroups(x, 1, 1);
        self
    }

    pub fn dispatch_2d(mut self, x: u32, y: u32) -> Self {
        self.setup_state();
        self.compute_pass.dispatch_workgroups(x, y, 1);
        self
    }

    pub fn dispatch_3d(mut self, x: u32, y: u32, z: u32) -> Self {
        self.setup_state();
        self.compute_pass.dispatch_workgroups(x, y, z);
        self
    }

    pub fn dispatch_indirect(mut self, buffer: &Buffer) -> Self {
        self.setup_state();
        self.compute_pass.dispatch_workgroups_indirect(buffer, 0);
        self
    }

    fn setup_state(&mut self) {
        // TODO: Compile and set pipeline, bind groups, push constants
    }
}
