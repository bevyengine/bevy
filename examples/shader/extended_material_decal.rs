//! Applies a decal to a standard material.

use bevy::{
    pbr::{
        exclude_standard_material_features, ExtendedMaterial, MaterialExtension,
        MaterialExtensionKey, MaterialExtensionPipeline, OpaqueRendererMethod,
        StandardMaterialExclusions,
    },
    prelude::*,
    render::mesh::MeshVertexBufferLayout,
    render::render_resource::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, DecalExtension>,
        >::default())
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_things)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, DecalExtension>>>,
    asset_server: ResMut<AssetServer>,
) {
    // sphere
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Sphere::new(1.0)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color_texture: Some(asset_server.load("textures/parallax_example/cube_color.png")),
                // can be used in forward or deferred mode.
                opaque_render_method: OpaqueRendererMethod::Auto,
                // in deferred mode, only the PbrInput can be modified (uvs, color and other material properties),
                // in forward mode, the output can also be modified after lighting is applied.
                // see the fragment shader `extended_material.wgsl` for more info.
                // Note: to run in deferred mode, you must also add a `DeferredPrepass` component to the camera and either
                // change the above to `OpaqueRendererMethod::Deferred` or add the `DefaultOpaqueRendererMethod` resource.
                ..Default::default()
            },
            extension: DecalExtension {
                decal: asset_server.load("textures/rpg/chars/vendor/generic-rpg-vendor.png"),
            },
        }),
        ..default()
    });

    // light
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 150_000.0,
                ..default()
            },
            ..default()
        },
        Rotate,
    ));

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Component)]
struct Rotate;

fn rotate_things(mut q: Query<&mut Transform, With<Rotate>>, time: Res<Time>) {
    for mut t in q.iter_mut() {
        t.translation = Vec3::new(
            time.elapsed_seconds().sin(),
            0.5,
            time.elapsed_seconds().cos(),
        ) * 4.0;
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct DecalExtension {
    // We need to ensure that the bindings of the base material and the extension do not conflict,
    // so we start from binding slot 100, leaving slots 0-99 for the base material.
    #[texture(100)]
    #[sampler(101)]
    decal: Handle<Image>,
}

impl MaterialExtension for DecalExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/extended_material_decal.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/extended_material_decal.wgsl".into()
    }

    fn specialize(
        _: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayout,
        _: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(exclude_standard_material_features(
            descriptor,
            StandardMaterialExclusions::BASE_COLOR,
        ))
    }
}
