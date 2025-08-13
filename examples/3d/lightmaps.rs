//! Rendering a scene with baked lightmaps.

use argh::FromArgs;
use bevy::{
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass},
    gltf::GltfMeshName,
    pbr::{DefaultOpaqueRendererMethod, Lightmap},
    prelude::*,
};

/// Demonstrates lightmaps
#[derive(FromArgs, Resource)]
struct Args {
    /// enables deferred shading
    #[argh(switch)]
    deferred: bool,
    /// enables bicubic filtering
    #[argh(switch)]
    bicubic: bool,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args: Args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight::NONE);

    if args.deferred {
        app.insert_resource(DefaultOpaqueRendererMethod::deferred());
    }

    app.insert_resource(args)
        .add_systems(Startup, setup)
        .add_systems(Update, add_lightmaps_to_meshes)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/CornellBox/CornellBox.glb"),
    )));

    let mut camera = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-278.0, 273.0, 800.0),
    ));

    if args.deferred {
        camera.insert((
            DepthPrepass,
            MotionVectorPrepass,
            DeferredPrepass,
            Msaa::Off,
        ));
    }
}

fn add_lightmaps_to_meshes(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<
        (Entity, &GltfMeshName, &MeshMaterial3d<StandardMaterial>),
        (With<Mesh3d>, Without<Lightmap>),
    >,
    args: Res<Args>,
) {
    let exposure = 250.0;
    for (entity, name, material) in meshes.iter() {
        if &**name == "large_box" {
            materials.get_mut(material).unwrap().lightmap_exposure = exposure;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("lightmaps/CornellBox-Large.zstd.ktx2"),
                bicubic_sampling: args.bicubic,
                ..default()
            });
            continue;
        }

        if &**name == "small_box" {
            materials.get_mut(material).unwrap().lightmap_exposure = exposure;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("lightmaps/CornellBox-Small.zstd.ktx2"),
                bicubic_sampling: args.bicubic,
                ..default()
            });
            continue;
        }

        if name.starts_with("cornell_box") {
            materials.get_mut(material).unwrap().lightmap_exposure = exposure;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("lightmaps/CornellBox-Box.zstd.ktx2"),
                bicubic_sampling: args.bicubic,
                ..default()
            });
            continue;
        }
    }
}
