use bevy::prelude::*;
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
    pub fn get_random_car<R: RngExt>(&self, rng: &mut R) -> Handle<WorldAsset> {
        self.cars[rng.random_range(0..self.cars.len())].clone()
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
