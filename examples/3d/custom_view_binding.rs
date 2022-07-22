use bevy::{
    asset::load_internal_asset,
    pbr::{
        queue_mesh_view_bind_groups, GetBinding, UserViewBindGroupLayoutEntry,
        UserViewBindingsEntries, UserViewBindingsLayouts, MESH_VIEW_USER_BINDINGS_HANDLE,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_resource::{
            encase::UniformBuffer, AsBindGroup, BindingType, Buffer, BufferBindingType,
            BufferInitDescriptor, BufferUsages, ShaderRef, ShaderStages, ShaderType,
        },
        renderer::RenderDevice,
        Extract, RenderApp, RenderStage,
    },
};

fn main() {
    let mut app = App::new();
    // add view bindings before adding default plugins
    CustomViewBindingPlugin::add_view_bindings(&mut app);
    app.add_plugins(DefaultPlugins)
        // plugin to populate the custom view binding
        .add_plugin(CustomViewBindingPlugin)
        // example material using the binding
        .add_plugin(MaterialPlugin::<CustomMaterial>::default())
        .add_startup_system(setup)
        .run();
}

// material to use the custom binding
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_view_binding_material.wgsl".into()
    }
}

#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct CustomMaterial {
    #[uniform(0)]
    color: Color,
}

// simple scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut custom_materials: ResMut<Assets<CustomMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // cube using the example material
    commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: custom_materials.add(CustomMaterial { color: Color::PINK }),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // cube using standard pbr material
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: standard_materials.add(StandardMaterial {
            base_color: Color::PINK,
            ..Default::default()
        }),
        transform: Transform::from_xyz(0.0, 0.5, -5.0),
        ..default()
    });

    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(ShaderType, AsBindGroup)]
struct CustomViewUniform {
    time: f32,
}

struct CustomViewUniformBuffer {
    buffer: Buffer,
}

impl GetBinding for CustomViewUniformBuffer {
    fn get_binding(&self) -> bevy::render::render_resource::BindingResource {
        self.buffer.as_entire_binding()
    }
}

pub struct CustomViewBindingPlugin;

impl CustomViewBindingPlugin {
    pub fn add_view_bindings(app: &mut App) {
        let mut user_bindings: Mut<UserViewBindingsLayouts> = app
            .world
            .get_resource_or_insert_with(UserViewBindingsLayouts::default);
        user_bindings.entries.push((
            "example custom view binding",
            UserViewBindGroupLayoutEntry {
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(CustomViewUniform::min_size()),
                },
            },
        ));
    }
}

impl Plugin for CustomViewBindingPlugin {
    fn build(&self, app: &mut App) {
        // override mesh view bindings
        // note: if an app uses multiple plugins with custom view bindings, you will need to create
        // a single view_binding .wgsl file containing all the bindings from the plugins, and override
        // the MESH_VIEW_USER_BINDINGS_HANDLE again after adding the plugins
        load_internal_asset!(
            app,
            MESH_VIEW_USER_BINDINGS_HANDLE,
            "custom_view_bindings.wgsl",
            Shader::from_wgsl
        );

        // extract example resource
        app.sub_app_mut(RenderApp)
            .add_system_to_stage(RenderStage::Extract, extract_time);

        // create the custom view binding buffer
        app.sub_app_mut(RenderApp).add_system_to_stage(
            RenderStage::Queue,
            queue_custom_view_binding
                // must be before queue_mesh_view_bind_groups which requests the user bindings
                .before(queue_mesh_view_bind_groups),
        );
    }
}

fn extract_time(mut commands: Commands, time: Extract<Res<Time>>) {
    commands.insert_resource(time.clone());
}

fn queue_custom_view_binding(
    mut entries: ResMut<UserViewBindingsEntries>,
    render_device: Res<RenderDevice>,
    time: Res<Time>,
) {
    let view_uniform = CustomViewUniform {
        time: time.seconds_since_startup() as f32,
    };

    let byte_buffer = vec![0u8; CustomViewUniform::min_size().get() as usize];
    let mut buffer = UniformBuffer::new(byte_buffer);
    buffer.write(&view_uniform).unwrap();

    let view_uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("custom view uniform"),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        contents: buffer.as_ref(),
    });

    let uniform_buffer = CustomViewUniformBuffer {
        buffer: view_uniform_buffer,
    };

    entries
        .entries
        .insert("example custom view binding", Box::new(uniform_buffer));
}
