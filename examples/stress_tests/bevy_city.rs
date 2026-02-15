//! A procedurally generated city

use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::Exposure,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    light::{atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight},
    pbr::AtmosphereSettings,
    post_process::bloom::Bloom,
    prelude::*,
};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        .add_systems(Startup, (setup, load_assets, setup_city.after(load_assets)))
        .run();
}

fn setup(mut commands: Commands, mut scattering_mediums: ResMut<Assets<ScatteringMedium>>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera::default(),
        Atmosphere::earthlike(scattering_mediums.add(ScatteringMedium::default())),
        AtmosphereSettings::default(),
        // The directional light illuminance used in this scene is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        Exposure { ev100: 13.0 },
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL,
        // Enables the atmosphere to drive reflections and ambient lighting (IBL) for this view
        AtmosphereEnvironmentMapLight::default(),
        Msaa::Off,
        TemporalAntiAliasing::default(),
    ));
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: false,
            illuminance: light_consts::lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::from_xyz(1.0, 0.15, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Resource)]
struct CityAssets {
    cars: Vec<Handle<Scene>>,
    crossroad: Handle<Scene>,
    road_straight: Handle<Scene>,
    high_density: Buildings,
    medium_density: Buildings,
    low_density: Buildings,
}

struct Buildings {
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<StandardMaterial>>,
}

impl Buildings {
    fn get_random_building<R: Rng>(
        &self,
        rng: &mut R,
    ) -> (Mesh3d, MeshMaterial3d<StandardMaterial>) {
        let mesh = self.meshes[rng.random_range(0..self.meshes.len())].clone();
        let material = self.materials[rng.random_range(0..self.materials.len())].clone();
        (Mesh3d(mesh), MeshMaterial3d(material))
    }
}

fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let base_url = "https://github.com/bevyengine/bevy_asset_files/raw/main/kenney";

    let cars = {
        // We need to trigger a load of the texture even if we never use it directly
        let car_texture: Handle<Image> =
            asset_server.load(format!("{base_url}/car-kit/Textures/colormap.png"));

        // TODO generate variations
        [
            "hatchback-sports",
            "suv",
            "suv-luxury",
            "sedan",
            "sedan-sports",
            "truck",
            "truck-flat",
            "van",
        ]
        .iter()
        .map(|t| {
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/car-kit/{t}.glb")))
        })
        .collect::<Vec<_>>()
    };

    let road_texture: Handle<Image> =
        asset_server.load(format!("{base_url}/city-kit-roads/Textures/colormap.png"));

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
    commands.insert_resource(CityAssets {
        cars,
        crossroad,
        road_straight,
        high_density,
        medium_density,
        low_density,
    });
}

fn setup_city(mut commands: Commands, assets: Res<CityAssets>) {
    let mut rng = rand::rng();
    let size = 3;
    for x in 0..size {
        for z in 0..size {
            let x = x as f32 * 5.5;
            let z = z as f32 * 4.0;
            commands.spawn((
                SceneRoot(assets.crossroad.clone()),
                // assets.high_density.get_random_building(&mut rng),
                Transform::from_xyz(x, 0.0, z),
            ));
        }
    }
    // for (i, car) in assets.cars.iter().enumerate() {
    //     commands.spawn((
    //         SceneRoot(car.clone()),
    //         Transform::from_xyz(i as f32 * 1.5, 0.0, 0.0),
    //     ));
    // }
}
