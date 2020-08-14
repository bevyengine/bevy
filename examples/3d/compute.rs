use bevy::prelude::*;
use bevy_core::{AsBytes};
use bevy_render::pipeline::{ ComputePipelineDescriptor, ComputePipelineCompiler, ComputePipelineSpecialization };
use bevy_render::shader::{ComputeShaderStages, ShaderStage};
use bevy_render::{dispatch::DispatchResource, renderer::{
    BufferId,
    RenderResourceContext,
    RenderResourceBindings, RenderResourceBinding, BufferUsage, BufferInfo
}};

fn main() {
    App::build()
        .add_resource::<ComputeState>(ComputeState::default())
        .add_plugin(bevy_type_registry::TypeRegistryPlugin::default())
        .add_plugin(bevy_asset::AssetPlugin::default())
        .add_plugin(bevy_window::WindowPlugin::default())
        .add_plugin(bevy_render::RenderPlugin::default())
        .add_plugin(bevy_winit::WinitPlugin::default())
        .add_plugin(bevy_wgpu::WgpuPlugin::default())
        .add_startup_system(setup.system())
        .add_startup_system(dispatch_system.system())
        .run();
}

#[derive(Default)]
struct ComputeState {
    storage_buffer: Option<BufferId>,
    pipeline_handle: Handle<ComputePipelineDescriptor>,
    shader: Handle<Shader>,
}

const COMPUTE_SHADER: &str = r#"
#version 450
layout(local_size_x = 1) in;

layout(set = 0, binding = 0) buffer PrimeIndices {
    uint[] indices;
}; // this is used as both input and output for convenience

// The Collatz Conjecture states that for any integer n:
// If n is even, n = n/2
// If n is odd, n = 3n+1
// And repeat this process for each new n, you will always eventually reach 1.
// Though the conjecture has not been proven, no counterexample has ever been found.
// This function returns how many times this recurrence needs to be applied to reach 1.
uint collatz_iterations(uint n) {
    uint i = 0;
    while(n != 1) {
        if (mod(n, 2) == 0) {
            n = n / 2;
        }
        else {
            n = (3 * n) + 1;
        }
        i++;
    }
    return i;
}

void main() {
    uint index = gl_GlobalInvocationID.x;
    indices[index] = collatz_iterations(indices[index]);
}
"#;

/// set up a simple 3D scene
fn setup(
    mut compute_state: ResMut<ComputeState>,
    mut pipelines: ResMut<Assets<ComputePipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut pipeline_compiler: ResMut<ComputePipelineCompiler>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
) {
    let render_resource_context = &**render_resource_context;

    let compute = shaders.add(Shader::from_glsl(ShaderStage::Compute, COMPUTE_SHADER));

    // Create pipeline..
    let pipeline = ComputePipelineDescriptor::new(
        ComputeShaderStages {
            compute: compute.clone(),
        }
    );
    let pipeline_handle = pipelines.add(pipeline);
    compute_state.shader = compute;

    let pipeline_handle = pipeline_compiler.compile_pipeline(
        render_resource_context,
        &mut pipelines,
        &mut shaders,
        pipeline_handle,
        &ComputePipelineSpecialization::default(),
    );

    compute_state.pipeline_handle = pipeline_handle;

    // Some prime data
    let prime_data: [u32; 4] = [0, 1, 7, 2];

    // Create Buffer
    let size = prime_data.len() * std::mem::size_of::<u32>();
    let storage_buffer = render_resource_context.create_buffer(BufferInfo {
        size,
        buffer_usage: BufferUsage::STORAGE | BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
        mapped_at_creation: true,
    });

    // Setup shader bindings for buffer.
    render_resource_bindings.set(
        "PrimeIndices",
        RenderResourceBinding::Buffer {
            buffer: storage_buffer,
            range: 0..size as u64,
            dynamic_index: None,
        },
    );

    render_resource_context.write_mapped_buffer(
        storage_buffer,
        0..size as u64,
        &mut |data, _renderer| {
        data.copy_from_slice(prime_data.as_bytes());
    });

    compute_state.storage_buffer = Some(storage_buffer);
}

fn dispatch_system(
    mut dispatch: ResMut<DispatchResource>,
    compute_state: Res<ComputeState>,
    pipelines: Res<Assets<ComputePipelineDescriptor>>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
) {
    let render_resource_context = &**render_resource_context;
    dispatch.set_pipeline(compute_state.pipeline_handle);

    // TODO: Move this out of here. We need to wrap a lot of this code to make it more user friendly.
    let pipeline_descriptor = pipelines
        .get(&compute_state.pipeline_handle).unwrap();
    let layout = pipeline_descriptor
        .get_layout().unwrap();
    
    render_resource_bindings.update_bind_groups(pipeline_descriptor.get_layout().unwrap(), render_resource_context);
    dbg!(&layout.bind_groups);
    for bind_group_descriptor in layout.bind_groups.iter() {
        if let Some(bind_group) =
        render_resource_bindings.get_descriptor_bind_group(bind_group_descriptor.id)
        {
            dispatch.set_bind_group(bind_group_descriptor.index, bind_group);
            break;
        }
    }

    dispatch.dispatch(4, 1, 1);
}
