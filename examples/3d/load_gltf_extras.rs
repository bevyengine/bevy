//! Loads and renders a glTF file as a scene, and list all the different `gltf_extras`.

use bevy::{
    gltf::{GltfExtras, GltfMaterialExtras, GltfMeshExtras, GltfSceneExtras},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, check_for_gltf_extras)
        .run();
}

#[derive(Component)]
struct ExampleDisplay;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn(DirectionalLight {
        shadows_enabled: true,
        ..default()
    });

    // a barebones scene containing one of each gltf_extra type
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/extras/gltf_extras.glb"),
    )));

    // a place to display the extras on screen
    commands.spawn((
        Text::default(),
        TextFont {
            font_size: 15.,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        ExampleDisplay,
    ));
}

fn check_for_gltf_extras(
    gltf_extras_per_entity: Query<(
        Entity,
        Option<&Name>,
        Option<&GltfSceneExtras>,
        Option<&GltfExtras>,
        Option<&GltfMeshExtras>,
        Option<&GltfMaterialExtras>,
    )>,
    mut display: Single<&mut Text, With<ExampleDisplay>>,
) {
    let mut gltf_extra_infos_lines: Vec<String> = vec![];

    for (id, name, scene_extras, extras, mesh_extras, material_extras) in
        gltf_extras_per_entity.iter()
    {
        if scene_extras.is_some()
            || extras.is_some()
            || mesh_extras.is_some()
            || material_extras.is_some()
        {
            let formatted_extras = format!(
                "Extras per entity {} ('Name: {}'):
    - scene extras:     {:?}
    - primitive extras: {:?}
    - mesh extras:      {:?}
    - material extras:  {:?}
                ",
                id,
                name.unwrap_or(&Name::default()),
                scene_extras,
                extras,
                mesh_extras,
                material_extras
            );
            gltf_extra_infos_lines.push(formatted_extras);
        }
        display.0 = gltf_extra_infos_lines.join("\n");
    }
}
