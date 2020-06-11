use bevy::{prelude::*, render::shader};

fn main() {
    App::build()
        .add_default_plugins()
        .add_asset::<MyMaterial>()
        .add_startup_system(setup.system())
        .add_system_to_stage(
            stage::POST_UPDATE,
            shader::asset_shader_def_system::<MyMaterial>.system(),
        )
        .run();
}

#[derive(RenderResources, ShaderDefs, Default)]
struct MyMaterial {
    pub color: Color,
    #[render_resources(ignore)]
    #[shader_def]
    pub always_red: bool,
}

const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(set = 0, binding = 0) uniform Camera {
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
layout(set = 1, binding = 1) uniform MyMaterial_color {
    vec4 color;
};
void main() {
    o_Target = color;

# ifdef MYMATERIAL_ALWAYS_RED
    o_Target = vec4(0.8, 0.0, 0.0, 1.0);
# endif
}
"#;

fn setup(
    command_buffer: &mut CommandBuffer,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // create new shader pipeline and add to main pass in Render Graph
    let pipeline_handle = {
        let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
            fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
        }));
        render_graph.add_system_node("my_material", AssetRenderResourcesNode::<MyMaterial>::new(true));
        pipeline_handle
    };

    // create materials
    let green_material = materials.add(MyMaterial {
        color: Color::rgb(0.0, 0.8, 0.0),
        always_red: false,
    });

    let red_material = materials.add(MyMaterial {
        color: Color::rgb(0.0, 0.0, 0.0),
        always_red: true,
    });

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    command_buffer
        .build()
        // cube
        .add_entity(MeshMaterialEntity::<MyMaterial> {
            mesh: cube_handle,
            render_pipelines: RenderPipelines {
                pipelines: vec![pipeline_handle],
                ..Default::default()
            },
            material: green_material,
            translation: Translation::new(-2.0, 0.0, 0.0),
            ..Default::default()
        })
        // cube
        .add_entity(MeshMaterialEntity::<MyMaterial> {
            mesh: cube_handle,
            render_pipelines: RenderPipelines {
                pipelines: vec![pipeline_handle],
                ..Default::default()
            },
            material: red_material,
            translation: Translation::new(2.0, 0.0, 0.0),
            ..Default::default()
        })
        // camera
        .add_entity(PerspectiveCameraEntity {
            transform: Transform::new_sync_disabled(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
