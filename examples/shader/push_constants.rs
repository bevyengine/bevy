use bevy::{
    core::AsBytes,
    prelude::*,
    reflect::TypeUuid,
    render::{
        draw::Drawable,
        mesh::{shape, Indices},
        pipeline::{
            BindingShaderStage, PipelineDescriptor, PipelineSpecialization, RenderPipeline,
        },
        shader::{ShaderStage, ShaderStages},
        RenderStage,
    },
    utils::HashSet,
    wgpu::{WgpuFeature, WgpuFeatures, WgpuLimits, WgpuOptions},
};

pub const CUSTOM_DRAWABLE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x137c75ab7e9ad8de);

/// This example illustrates how to create a custom material asset that uses "shader defs" and a shader that uses that material.
/// In Bevy, "shader defs" are a way to selectively enable parts of a shader based on values set in a component or asset.
fn main() {
    App::build()
        .insert_resource(WgpuOptions {
            features: WgpuFeatures {
                features: vec![WgpuFeature::PushConstants],
            },
            limits: WgpuLimits {
                max_push_constant_size: 128,
                ..Default::default()
            },
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .register_type::<CustomDrawable>()
        .add_startup_system(setup.system())
        .add_system_to_stage(RenderStage::Draw, draw_custom_drawable.system())
        .run();
}

const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};
void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450
layout(location = 0) out vec4 o_Target;
layout(push_constant) uniform PushConstants {
    vec3 Color;
};
void main() {
    o_Target = vec4(Color, 1.0);
}
"#;

#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
struct CustomDrawable;

impl Drawable for CustomDrawable {
    fn draw(
        &mut self,
        draw: &mut Draw,
        context: &mut bevy::render::draw::DrawContext,
    ) -> Result<(), bevy::render::draw::DrawError> {
        context.set_push_constants(
            draw,
            BindingShaderStage::FRAGMENT,
            0,
            Into::<[f32; 4]>::into(Color::BLUE).as_bytes().to_vec(),
        )
    }
}

fn draw_custom_drawable(
    mut draw_context: bevy::render::draw::DrawContext,
    msaa: Res<Msaa>,
    meshes: Res<Assets<Mesh>>,
    mut query: Query<(
        &mut Draw,
        &mut RenderPipelines,
        &Handle<Mesh>,
        &mut CustomDrawable,
        &Visible,
    )>,
) {
    for (mut draw, mut render_pipelines, mesh_handle, mut custom_drawable, visible) in
        query.iter_mut()
    {
        if !visible.is_visible {
            continue;
        }

        // don't render if the mesh isn't loaded yet
        let mesh = if let Some(mesh) = meshes.get(mesh_handle) {
            mesh
        } else {
            return;
        };

        // clear out any previous render_commands
        // TODO prevent draw_render_pipelines_system from running for this
        draw.clear_render_commands();

        let mut render_pipeline = RenderPipeline::specialized(
            CUSTOM_DRAWABLE_PIPELINE_HANDLE.typed(),
            PipelineSpecialization {
                sample_count: msaa.samples,
                strip_index_format: None,
                shader_specialization: Default::default(),
                primitive_topology: mesh.primitive_topology(),
                dynamic_bindings: render_pipelines
                    .bindings
                    .iter_dynamic_bindings()
                    .map(|name| name.to_string())
                    .collect::<HashSet<String>>(),
                vertex_buffer_layout: mesh.get_vertex_buffer_layout(),
            },
        );
        render_pipeline.dynamic_bindings_generation =
            render_pipelines.bindings.dynamic_bindings_generation();

        draw_context
            .set_pipeline(
                &mut draw,
                &render_pipeline.pipeline,
                &render_pipeline.specialization,
            )
            .unwrap();
        custom_drawable.draw(&mut *draw, &mut draw_context).unwrap();
        draw_context
            .set_bind_groups_from_bindings(&mut draw, &mut [&mut render_pipelines.bindings])
            .unwrap();
        draw_context
            .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
            .unwrap();

        match mesh.indices() {
            Some(Indices::U32(indices)) => draw.draw_indexed(0..indices.len() as u32, 0, 0..1),
            Some(Indices::U16(indices)) => draw.draw_indexed(0..indices.len() as u32, 0, 0..1),
            None => draw.draw(0..mesh.count_vertices() as u32, 0..1),
        };
    }
}

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Create a new shader pipeline
    let pipeline_descriptor = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    });
    let pipeline_handle = pipelines.set(CUSTOM_DRAWABLE_PIPELINE_HANDLE, pipeline_descriptor);

    commands
        // cube
        .spawn(MeshBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle,
            )]),
            transform: Transform::from_xyz(-2.0, 0.0, 0.0),
            ..Default::default()
        })
        .with(CustomDrawable)
        // camera
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_xyz(3.0, 5.0, -8.0).looking_at(Vec3::default(), Vec3::Y),
            ..Default::default()
        });
}
