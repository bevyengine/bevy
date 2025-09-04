//! Showcases how to change the material of a `Scene` spawned from a Gltf

use bevy::{
    audio::AudioPlugin, color::palettes, gltf::GltfMaterialName, prelude::*,
    scene::SceneInstanceReady,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.build().disable::<AudioPlugin>())
        .add_systems(Startup, setup_scene)
        .add_observer(change_material)
        .run();
}

/// This is added to a [`SceneRoot`] and will cause the [`StandardMaterial::base_color`]
/// of all materials to be overwritten
#[derive(Component)]
struct ColorOverride(Color);

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1., 2.5).looking_at(Vec3::new(0., 0.25, 0.), Dir3::Y),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(0., 1., 0.25).looking_at(Vec3::ZERO, Dir3::Y),
    ));

    // FlightHelmet handle
    let flight_helmet = asset_server
        .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));
    // This model will keep its original materials
    commands.spawn(SceneRoot(flight_helmet.clone()));
    // This model will be tinted red
    commands.spawn((
        SceneRoot(flight_helmet.clone()),
        Transform::from_xyz(-1.25, 0., 0.),
        ColorOverride(palettes::tailwind::RED_300.into()),
    ));
    // This model will be tinted green
    commands.spawn((
        SceneRoot(flight_helmet),
        Transform::from_xyz(1.25, 0., 0.),
        ColorOverride(palettes::tailwind::GREEN_300.into()),
    ));
}

fn change_material(
    event: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    color_override: Query<&ColorOverride>,
    mesh_materials: Query<(&MeshMaterial3d<StandardMaterial>, &GltfMaterialName)>,
    mut asset_materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("processing Scene Entity: {}", event.entity());
    // Iterate over all children recursively
    for descendant in children.iter_descendants(event.entity()) {
        // Get the material id and name which were created from the glTF file information
        let Ok((id, material_name)) = mesh_materials.get(descendant) else {
            continue;
        };
        // Get the material of the descendant
        let Some(material) = asset_materials.get_mut(id.id()) else {
            continue;
        };

        // match on the material name, modifying the materials as necessary
        match material_name.0.as_str() {
            "LeatherPartsMat" => {
                info!("editing LeatherPartsMat to use ColorOverride tint");
                // Get the `ColorOverride` of the entity, if it does not have a color override, skip
                let Ok(color_override) = color_override.get(event.entity()) else {
                    continue;
                };
                // Create a copy of the material and override base color
                // If you intend on creating multiple models with the same tint, it
                // is best to cache the handle somewhere, as having multiple materials
                // that are identical is expensive
                let mut new_material = material.clone();
                new_material.base_color = color_override.0;

                // Override `MeshMaterial3d` with new material
                commands
                    .entity(descendant)
                    .insert(MeshMaterial3d(asset_materials.add(new_material)));
            }
            name => {
                info!("not replacing: {name}");
            }
        }
    }
}
