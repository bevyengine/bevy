//! Demonstrates using a custom extension to the `StandardMaterial` to modify the results of the builtin pbr shader.

use bevy::{
    color::palettes::basic::RED,
    pbr::{ExtendedMaterial, MaterialExtension, OpaqueRendererMethod},
    prelude::*,
    render::render_resource::*,
    shader::ShaderRef,
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/extended_material.wgsl";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, MyExtension>,
        >::default())
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_things)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, MyExtension>>>,
) {
    // sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: RED.into(),
                // can be used in forward or deferred mode
                opaque_render_method: OpaqueRendererMethod::Auto,
                // in deferred mode, only the PbrInput can be modified (uvs, color and other material properties),
                // in forward mode, the output can also be modified after lighting is applied.
                // see the fragment shader `extended_material.wgsl` for more info.
                // Note: to run in deferred mode, you must also add a `DeferredPrepass` component to the camera and either
                // change the above to `OpaqueRendererMethod::Deferred` or add the `DefaultOpaqueRendererMethod` resource.
                ..Default::default()
            },
            extension: MyExtension::new(1),
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        Rotate,
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Component)]
struct Rotate;

fn rotate_things(mut q: Query<&mut Transform, With<Rotate>>, time: Res<Time>) {
    for mut t in &mut q {
        t.rotate_y(time.delta_secs());
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
struct MyExtension {
    // We need to ensure that the bindings of the base material and the extension do not conflict,
    // so we start from binding slot 100, leaving slots 0-99 for the base material.
    #[uniform(100)]
    quantize_steps: u32,
    // Web examples WebGL2 support: structs must be 16 byte aligned.
    #[cfg(feature = "webgl2")]
    #[uniform(100)]
    _webgl2_padding_8b: u32,
    #[cfg(feature = "webgl2")]
    #[uniform(100)]
    _webgl2_padding_12b: u32,
    #[cfg(feature = "webgl2")]
    #[uniform(100)]
    _webgl2_padding_16b: u32,
}
impl MyExtension {
    fn new(quantize_steps: u32) -> Self {
        Self {
            quantize_steps,
            ..default()
        }
    }
}

impl MaterialExtension for MyExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
