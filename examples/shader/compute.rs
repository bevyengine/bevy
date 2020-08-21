use bevy::prelude::*;
use bevy_render::{
    dispatch::Dispatch,
    pipeline::{
        ComputePipeline, ComputePipelineDescriptor, ComputePipelineSpecialization, ComputePipelines,
    },
    render_graph::{base::node::COMPUTE_PASS, AssetRenderResourcesNode, RenderGraph},
    renderer::RenderResources,
    shader::{ComputeShaderStages, ShaderStage},
};

fn main() {
    App::build()
        .add_default_plugins()
        .add_asset::<PrimeIndices>()
        .add_startup_system(setup.system())
        .run();
}

const COMPUTE_SHADER: &str = r#"
#version 450
layout(local_size_x = 1) in;

layout(set = 0, binding = 0) buffer PrimeIndices_indices {
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
    if (n == 0) {
        return n;
    }
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
#[derive(Default, RenderResources)]
struct PrimeIndices {
    #[render_resources(buffer)]
    indices: Vec<u32>,
}

#[derive(Default, Bundle)]
struct ComputeComponents {
    compute_pipelines: ComputePipelines,
    dispatch: Dispatch,
}

// Setup our compute pipeline and a "dispatch" component/entity that will dispatch the shader.
fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<ComputePipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut render_graph: ResMut<RenderGraph>,
    mut prime_indices: ResMut<Assets<PrimeIndices>>,
) {
    let compute = shaders.add(Shader::from_glsl(ShaderStage::Compute, COMPUTE_SHADER));

    // Create pipeline..
    let pipeline = ComputePipelineDescriptor::new(ComputeShaderStages { compute });
    let pipeline_handle = pipelines.add(pipeline);

    render_graph.add_system_node(
        "prime_indices",
        AssetRenderResourcesNode::<PrimeIndices>::new(false),
    );
    render_graph
        .add_node_edge("prime_indices", COMPUTE_PASS)
        .unwrap();

    // Some prime data
    let prime_data = vec![1, 2, 3, 4];
    let data_count = prime_data.len();

    let primes_handle = prime_indices.add(PrimeIndices {
        indices: prime_data,
    });

    commands
        .spawn(ComputeComponents {
            compute_pipelines: ComputePipelines::from_pipelines(vec![
                ComputePipeline::specialized(
                    pipeline_handle,
                    ComputePipelineSpecialization {
                        ..Default::default()
                    },
                ),
            ]),
            dispatch: Dispatch {
                only_once: false,
                work_group_size_x: data_count as u32,
                ..Default::default()
            },
        })
        .with(primes_handle);
}
