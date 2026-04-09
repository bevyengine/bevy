use bevy::{ecs::system::SystemState, prelude::*};
use rand::RngExt;

const BASE_URL: &str = "https://github.com/bevyengine/bevy_asset_files/raw/main/kenney";

pub fn strip_base_url(path: String) -> String {
    path.strip_prefix(BASE_URL)
        .map(|s| s.trim_start_matches('/').to_string())
        .unwrap_or(path)
}

#[derive(Resource)]
pub struct CityAssets {
    pub untyped_assets: Vec<UntypedHandle>,
    pub cars: Vec<Handle<WorldAsset>>,
    pub car_meshes: Vec<Handle<Mesh>>,
    pub car_material: Handle<StandardMaterial>,
    pub crossroad: Handle<WorldAsset>,
    pub road_straight: Handle<WorldAsset>,
    pub high_density: Buildings,
    pub medium_density: Buildings,
    pub low_density: Buildings,
    pub ground_tile: (
        Handle<Mesh>,
        Handle<StandardMaterial>,
        Handle<StandardMaterial>,
    ),
    pub tree_small: Handle<WorldAsset>,
    pub tree_large: Handle<WorldAsset>,
    pub path_stones_long: Handle<WorldAsset>,
    pub fence: Handle<WorldAsset>,
}

impl CityAssets {
    pub fn get_random_car<R: RngExt>(
        &self,
        rng: &mut R,
    ) -> (Mesh3d, MeshMaterial3d<StandardMaterial>) {
        let mesh = self.car_meshes[rng.random_range(0..self.car_meshes.len())].clone();
        (Mesh3d(mesh), MeshMaterial3d(self.car_material.clone()))
    }
}

pub struct Buildings {
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<StandardMaterial>>,
}

impl Buildings {
    pub fn get_random_building<R: RngExt>(
        &self,
        rng: &mut R,
    ) -> (Mesh3d, MeshMaterial3d<StandardMaterial>) {
        let mesh = self.meshes[rng.random_range(0..self.meshes.len())].clone();
        let material = self.materials[rng.random_range(0..self.materials.len())].clone();
        (Mesh3d(mesh), MeshMaterial3d(material))
    }
}

pub fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let base_url = BASE_URL;

    let mut untyped_assets = vec![];
    /// Wraps asset_server.load_asset to automatically track all the assets that are being loaded
    macro_rules! load_asset {
        ($path:expr) => {{
            let handle = asset_server.load($path);
            untyped_assets.push(handle.clone().untyped());
            handle
        }};
    }

    let car_texture: Handle<Image> =
        load_asset!(format!("{base_url}/car-kit/Textures/colormap.png"));
    let car_material = materials.add(StandardMaterial {
        base_color_texture: Some(car_texture),
        ..Default::default()
    });

    let cars = {
        // TODO generate color variations
        [
            "hatchback-sports",
            "suv",
            "suv-luxury",
            "sedan",
            "sedan-sports",
            "truck",
            "truck-flat",
            "van",
            "delivery",
            "delivery-flat",
            "taxi",
            "garbage-truck",
            "ambulance",
            "police",
            "firetruck",
        ]
        .iter()
        .map(|t| {
            load_asset!(GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/car-kit/{t}.glb")))
        })
        .collect::<Vec<_>>()
    };

    let crossroad = load_asset!(GltfAssetLabel::Scene(0)
        .from_asset(format!("{base_url}/city-kit-roads/road-crossroad-path.glb")));
    let road_straight =
        load_asset!(GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-roads/road-straight.glb")));

    let high_density = {
        let materials = ["colormap", "variation-a", "variation-b"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(load_asset!(format!(
                        "{base_url}/city-kit-commercial/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();

        let mut meshes = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|t| {
                load_asset!(GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(format!(
                    "{base_url}/city-kit-commercial/building-skyscraper-{t}.glb"
                )))
            })
            .collect::<Vec<_>>();
        meshes.extend(["m", "l"].iter().map(|t| {
            load_asset!(GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset(format!("{base_url}/city-kit-commercial/building-{t}.glb")))
        }));

        Buildings { meshes, materials }
    };

    let medium_density = {
        let materials = ["colormap", "variation-a", "variation-b"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(load_asset!(format!(
                        "{base_url}/city-kit-commercial/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let meshes = ["a", "b", "c", "d", "f", "g", "h"]
            .iter()
            .map(|t| {
                load_asset!(GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(format!("{base_url}/city-kit-commercial/building-{t}.glb")))
            })
            .collect::<Vec<_>>();

        Buildings { meshes, materials }
    };
    let low_density = {
        let materials = ["colormap", "variation-a", "variation-b", "variation-c"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(load_asset!(format!(
                        "{base_url}/city-kit-suburban/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let meshes = ["b", "c", "d", "e", "f", "g", "h", "i", "k", "l", "o", "u"]
            .iter()
            .map(|t| {
                load_asset!(GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(format!(
                    "{base_url}/city-kit-suburban/building-type-{t}.glb"
                )))
            })
            .collect::<Vec<_>>();

        Buildings { meshes, materials }
    };

    let ground_tile = {
        let mesh = load_asset!(GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset(format!("{base_url}/city-kit-roads/tile-low.glb")));
        let default_material: Handle<StandardMaterial> = load_asset!(format!(
            "{base_url}/city-kit-roads/tile-low.glb#{}/std",
            GltfAssetLabel::DefaultMaterial
        ));
        let grass_material =
            materials.add(StandardMaterial::from_color(Color::srgb_u8(97, 203, 139)));

        (mesh, default_material, grass_material)
    };

    let tree_small: Handle<WorldAsset> =
        load_asset!(GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-suburban/tree-small.glb")));
    let tree_large: Handle<WorldAsset> =
        load_asset!(GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-suburban/tree-large.glb")));

    let path_stones_long: Handle<WorldAsset> = load_asset!(GltfAssetLabel::Scene(0)
        .from_asset(format!("{base_url}/city-kit-suburban/path-stones-long.glb")));

    let fence: Handle<WorldAsset> = load_asset!(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-suburban/fence.glb"))
    );

    commands.insert_resource(CityAssets {
        untyped_assets,
        cars,
        car_meshes: vec![],
        car_material,
        crossroad,
        road_straight,
        high_density,
        medium_density,
        low_density,
        ground_tile,
        tree_small,
        tree_large,
        path_stones_long,
        fence,
    });
}

/// Merge the meshes of all the cars gltf into a single mesh per car.
///
/// The asset pack we are using uses a separate mesh for each tire of the car and some also have
/// doors as separate meshes. This is useful if you want to animate these element individually but
/// in this scene we don't need to do that. Having multiple meshes for a single car means we need
/// to run transform propagation on all these meshes and it will also generate even more indirect
/// commands for each of those meshes.
pub fn merge_car_meshes(
    city_assets: &mut CityAssets,
    world_assets: &mut Assets<WorldAsset>,
    meshes: &mut Assets<Mesh>,
) {
    for car_scene in &city_assets.cars {
        let Some(merged) = merge_world_asset(world_assets, meshes, car_scene) else {
            continue;
        };
        city_assets.car_meshes.push(meshes.add(merged));
    }
}

/// Merge an entire scene into a single mesh
fn merge_world_asset(
    world_assets: &mut Assets<WorldAsset>,
    meshes: &mut Assets<Mesh>,
    scene_handle: &Handle<WorldAsset>,
) -> Option<Mesh> {
    let mut scene = world_assets.get_mut(scene_handle)?;
    let mut merged: Option<Mesh> = None;

    let mut system_state = SystemState::<TransformHelper>::new(&mut scene.world);
    let helper = system_state.get(&scene.world).ok()?;

    for entity_ref in scene.world.iter_entities() {
        let Some(mesh) = entity_ref
            .get::<Mesh3d>()
            .and_then(|mesh3d| meshes.get(mesh3d))
        else {
            continue;
        };
        let Ok(global_transform) = helper.compute_global_transform(entity_ref.id()) else {
            continue;
        };
        let transform = global_transform.compute_transform();
        let transformed = mesh.clone().transformed_by(transform);
        match &mut merged {
            Some(mesh) => {
                let _ = mesh.merge(&transformed);
            }
            None => merged = Some(transformed),
        }
    }
    merged
}
