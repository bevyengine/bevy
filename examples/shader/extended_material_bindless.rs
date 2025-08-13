//! Demonstrates bindless `ExtendedMaterial`.

use std::f32::consts::FRAC_PI_2;

use bevy::{
    color::palettes::{css::RED, tailwind::GRAY_600},
    mesh::{SphereKind, SphereMeshBuilder},
    pbr::{ExtendedMaterial, MaterialExtension, MeshMaterial3d},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    utils::default,
};

/// The path to the example material shader.
static SHADER_ASSET_PATH: &str = "shaders/extended_material_bindless.wgsl";

/// The example bindless material extension.
///
/// As usual for material extensions, we need to avoid conflicting with both the
/// binding numbers and bindless indices of the [`StandardMaterial`], so we
/// start both values at 100 and 50 respectively.
///
/// The `#[data(50, ExampleBindlessExtensionUniform, binding_array(101))]`
/// attribute specifies that the plain old data
/// [`ExampleBindlessExtensionUniform`] will be placed into an array with
/// binding 100 and will occupy index 50 in the
/// `ExampleBindlessExtendedMaterialIndices` structure. (See the shader for the
/// definition of that structure.) That corresponds to the following shader
/// declaration:
///
/// ```wgsl
/// @group(2) @binding(100) var<storage> example_extended_material_indices:
///     array<ExampleBindlessExtendedMaterialIndices>;
/// ```
///
/// The `#[bindless(index_table(range(50..53), binding(100)))]` attribute
/// specifies that this material extension should be bindless. The `range`
/// subattribute specifies that this material extension should have its own
/// index table covering bindings 50, 51, and 52. The `binding` subattribute
/// specifies that the extended material index table should be bound to binding
/// 100. This corresponds to the following shader declarations:
///
/// ```wgsl
/// struct ExampleBindlessExtendedMaterialIndices {
///     material: u32,                      // 50
///     modulate_texture: u32,              // 51
///     modulate_texture_sampler: u32,      // 52
/// }
///
/// @group(2) @binding(100) var<storage> example_extended_material_indices:
///     array<ExampleBindlessExtendedMaterialIndices>;
/// ```
///
/// We need to use the `index_table` subattribute because the
/// [`StandardMaterial`] bindless index table is bound to binding 0 by default.
/// Thus we need to specify a different binding so that our extended bindless
/// index table doesn't conflict.
#[derive(Asset, Clone, Reflect, AsBindGroup)]
#[data(50, ExampleBindlessExtensionUniform, binding_array(101))]
#[bindless(index_table(range(50..53), binding(100)))]
struct ExampleBindlessExtension {
    /// The color we're going to multiply the base color with.
    modulate_color: Color,
    /// The image we're going to multiply the base color with.
    #[texture(51)]
    #[sampler(52)]
    modulate_texture: Option<Handle<Image>>,
}

/// The GPU-side data structure specifying plain old data for the material
/// extension.
#[derive(Clone, Default, ShaderType)]
struct ExampleBindlessExtensionUniform {
    /// The GPU representation of the color we're going to multiply the base
    /// color with.
    modulate_color: Vec4,
}

impl MaterialExtension for ExampleBindlessExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

impl<'a> From<&'a ExampleBindlessExtension> for ExampleBindlessExtensionUniform {
    fn from(material_extension: &'a ExampleBindlessExtension) -> Self {
        // Convert the CPU `ExampleBindlessExtension` structure to its GPU
        // format.
        ExampleBindlessExtensionUniform {
            modulate_color: LinearRgba::from(material_extension.modulate_color).to_vec4(),
        }
    }
}

/// The entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, ExampleBindlessExtension>,
        >::default())
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_sphere)
        .run();
}

/// Creates the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ExampleBindlessExtension>>>,
) {
    // Create a gray sphere, modulated with a red-tinted Bevy logo.
    commands.spawn((
        Mesh3d(meshes.add(SphereMeshBuilder::new(
            1.0,
            SphereKind::Uv {
                sectors: 20,
                stacks: 20,
            },
        ))),
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: GRAY_600.into(),
                ..default()
            },
            extension: ExampleBindlessExtension {
                modulate_color: RED.into(),
                modulate_texture: Some(asset_server.load("textures/uv_checker_bw.png")),
            },
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Create a light.
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Create a camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn rotate_sphere(mut meshes: Query<&mut Transform, With<Mesh3d>>, time: Res<Time>) {
    for mut transform in &mut meshes {
        transform.rotation =
            Quat::from_euler(EulerRot::YXZ, -time.elapsed_secs(), FRAC_PI_2 * 3.0, 0.0);
    }
}
