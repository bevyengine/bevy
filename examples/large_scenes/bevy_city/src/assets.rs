use bevy::{color::palettes::css::WHITE, prelude::*};
use rand::RngExt;

#[derive(Resource)]
pub struct CityAssets {
    pub cars: Vec<Handle<Scene>>,
    pub crossroad: Handle<Scene>,
    pub road_straight: Handle<Scene>,
    pub high_density: Buildings,
    pub medium_density: Buildings,
    pub low_density: Buildings,
    pub ground_tile: (
        Handle<Mesh>,
        Handle<StandardMaterial>,
        Handle<StandardMaterial>,
    ),
    pub tree_small: Handle<Scene>,
    pub tree_large: Handle<Scene>,
    pub path_stones_long: Handle<Scene>,
    pub fence: Handle<Scene>,
}

impl CityAssets {
    pub fn get_random_car<R: RngExt>(&self, rng: &mut R) -> Handle<Scene> {
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
    let base_url = "https://github.com/bevyengine/bevy_asset_files/raw/main/kenney";

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
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/car-kit/{t}.glb")))
        })
        .collect::<Vec<_>>()
    };

    let crossroad = asset_server.load(
        GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-roads/road-crossroad-path.glb")),
    );
    let road_straight = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-roads/road-straight.glb")),
    );

    let high_density = {
        let materials = ["colormap", "variation-a", "variation-b"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load(format!(
                        "{base_url}/city-kit-commercial/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();

        let mut meshes = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|t| {
                asset_server.load(
                    GltfAssetLabel::Primitive {
                        mesh: 0,
                        primitive: 0,
                    }
                    .from_asset(format!(
                        "{base_url}/city-kit-commercial/building-skyscraper-{t}.glb"
                    )),
                )
            })
            .collect::<Vec<_>>();
        meshes.extend(["m", "l"].iter().map(|t| {
            asset_server.load(
                GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(format!("{base_url}/city-kit-commercial/building-{t}.glb")),
            )
        }));

        Buildings { meshes, materials }
    };

    let medium_density = {
        let materials = ["colormap", "variation-a", "variation-b"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load(format!(
                        "{base_url}/city-kit-commercial/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let meshes = ["a", "b", "c", "d", "f", "g", "h"]
            .iter()
            .map(|t| {
                asset_server.load(
                    GltfAssetLabel::Primitive {
                        mesh: 0,
                        primitive: 0,
                    }
                    .from_asset(format!("{base_url}/city-kit-commercial/building-{t}.glb")),
                )
            })
            .collect::<Vec<_>>();

        Buildings { meshes, materials }
    };
    let low_density = {
        let materials = ["colormap", "variation-a", "variation-b", "variation-c"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load(format!(
                        "{base_url}/city-kit-suburban/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let meshes = ["b", "c", "d", "e", "f", "g", "h", "i", "k", "l", "o", "u"]
            .iter()
            .map(|t| {
                asset_server.load(
                    GltfAssetLabel::Primitive {
                        mesh: 0,
                        primitive: 0,
                    }
                    .from_asset(format!(
                        "{base_url}/city-kit-suburban/building-type-{t}.glb"
                    )),
                )
            })
            .collect::<Vec<_>>();

        Buildings { meshes, materials }
    };

    let ground_tile = {
        let mesh = asset_server.load(
            GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset(format!("{base_url}/city-kit-roads/tile-low.glb")),
        );
        // TODO use this once https://github.com/bevyengine/bevy/pull/22943 is merged
        // let default_material: Handle<StandardMaterial> = asset_server.load(format!(
        //     "ground_tile/tile-low.glb#{}/std",
        //     GltfAssetLabel::DefaultMaterial
        // ));
        let white_material = materials.add(StandardMaterial::from_color(WHITE));
        let grass_material =
            materials.add(StandardMaterial::from_color(Color::srgb_u8(97, 203, 139)));

        (mesh, white_material, grass_material)
    };

    let tree_small: Handle<Scene> = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-suburban/tree-small.glb")),
    );
    let tree_large: Handle<Scene> = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-suburban/tree-large.glb")),
    );

    let path_stones_long: Handle<Scene> = asset_server.load(
        GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-suburban/path-stones-long.glb")),
    );

    let fence: Handle<Scene> = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-suburban/fence.glb")),
    );

    commands.insert_resource(CityAssets {
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
