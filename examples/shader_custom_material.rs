use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .run();
}

#[derive(Uniforms, Default)]
struct MyMaterial {
    pub color: Color,
}

fn setup(world: &mut World, resources: &mut Resources) {
    // create new shader pipeline and add to main pass in Render Graph
    let pipeline_handle = {
        let mut pipelines = resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();
        let mut shaders = resources.get_mut::<AssetStorage<Shader>>().unwrap();

        let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                r#"
                #version 450
                layout(location = 0) in vec3 Vertex_Position;
                layout(set = 0, binding = 0) uniform Camera {
                    mat4 ViewProj;
                };
                layout(set = 1, binding = 0) uniform Object {
                    mat4 Model;
                };
                void main() {
                    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
                }
            "#,
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                r#"
                #version 450
                layout(location = 0) out vec4 o_Target;
                layout(set = 1, binding = 1) uniform MyMaterial_color {
                    vec4 color;
                };
                void main() {
                    o_Target = color;
                }
            "#,
            ))),
        }));
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_system_node_named(
            "my_material",
            AssetUniformNode::<MyMaterial>::new(true),
            resources,
        );
        let main_pass: &mut PassNode = render_graph.get_node_mut("main_pass").unwrap();
        main_pass.add_pipeline(
            pipeline_handle,
            vec![Box::new(draw_target::AssignedMeshesDrawTarget)],
        );
        pipeline_handle
    };

    // create materials
    let mut material_storage = AssetStorage::<MyMaterial>::new();
    let material = material_storage.add(MyMaterial {
        color: Color::rgb(0.0, 0.8, 0.0),
    });
    resources.insert(material_storage);

    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let cube_handle = mesh_storage.add(Mesh::from(shape::Cube));

    world
        .build()
        // cube
        .add_entity(MeshMaterialEntity::<MyMaterial> {
            mesh: cube_handle,
            renderable: Renderable {
                pipelines: vec![pipeline_handle],
                ..Default::default()
            },
            material,
            translation: Translation::new(0.0, 0.0, 0.0),
            ..Default::default()
        })
        // camera
        .add_entity(CameraEntity {
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
