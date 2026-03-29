use bevy::{
    asset::RenderAssetUsages,
    camera::{primitives::MeshAabb, visibility::VisibilityRange},
    mesh::{Indices, PrimitiveTopology},
    platform::collections::HashMap,
    prelude::*,
};
use rand::RngExt;

const BASE_URL: &str = "https://github.com/bevyengine/bevy_asset_files/raw/main/kenney";

pub fn strip_base_url(path: String) -> String {
    path.strip_prefix(BASE_URL)
        .map(|s| s.trim_start_matches('/').to_string())
        .unwrap_or(path)
}

use crate::{Args, Car};

#[derive(Resource)]
pub struct CityAssets {
    pub untyped_assets: Vec<UntypedHandle>,
    pub cars: Vec<Handle<Scene>>,
    pub car_lod: (Handle<Mesh>, Handle<StandardMaterial>),
    pub crossroad: Handle<Scene>,
    pub road_straight: Handle<Scene>,
    pub high_density: Buildings,
    pub medium_density: Buildings,
    pub low_density: Buildings,
    pub low_density_lod: (Rect, Rect),
    pub medium_density_lod: (Rect, Rect),
    pub high_density_lod: (Rect, Rect),
    pub ground_tile: (
        Handle<Mesh>,
        Handle<StandardMaterial>,
        Handle<StandardMaterial>,
    ),
    pub tree_small: Handle<Scene>,
    pub tree_large: Handle<Scene>,
    pub tree_small_lod: (Handle<Mesh>, Handle<StandardMaterial>),
    pub tree_large_lod: (Handle<Mesh>, Handle<StandardMaterial>),
    pub path_stones_long: Handle<Scene>,
    pub fence: Handle<Scene>,
    pub visibility_ranges: Vec<VisibilityRange>,
    pub car_visibility_ranges: Vec<VisibilityRange>,
}

impl CityAssets {
    pub fn get_random_car<R: RngExt>(&self, rng: &mut R) -> Handle<Scene> {
        self.cars[rng.random_range(0..self.cars.len())].clone()
    }

    pub fn spawn_tree_small(&self, commands: &mut ChildSpawnerCommands, transform: Transform) {
        commands
            .spawn((transform, Visibility::default()))
            .with_children(|commands| {
                commands.spawn((
                    SceneRoot(self.tree_small.clone()),
                    Transform::default(),
                    self.visibility_ranges[0].clone(),
                ));
                commands.spawn((
                    Mesh3d(self.tree_small_lod.0.clone()),
                    MeshMaterial3d(self.tree_small_lod.1.clone()),
                    Transform::from_xyz(0.0, 0.3, 0.0),
                    self.visibility_ranges[1].clone(),
                ));
            });
    }

    pub fn spawn_tree_large(&self, commands: &mut ChildSpawnerCommands, transform: Transform) {
        commands
            .spawn((transform, Visibility::default()))
            .with_children(|commands| {
                commands.spawn((
                    SceneRoot(self.tree_large.clone()),
                    Transform::default(),
                    self.visibility_ranges[0].clone(),
                ));
                commands.spawn((
                    Mesh3d(self.tree_large_lod.0.clone()),
                    MeshMaterial3d(self.tree_large_lod.1.clone()),
                    Transform::from_xyz(0.0, 0.4, 0.0),
                    self.visibility_ranges[1].clone(),
                ));
            });
    }

    pub fn spawn_low_density_building<R: RngExt>(
        &self,
        commands: &mut ChildSpawnerCommands,
        rng: &mut R,
        transform: Transform,
    ) {
        commands
            .spawn((transform, Visibility::default()))
            .with_children(|commands| {
                let (mesh, material) = self.low_density.get_random_building(rng);
                commands.spawn((
                    mesh.clone(),
                    material.clone(),
                    Transform::default(),
                    self.visibility_ranges[0].clone(),
                ));
                commands.spawn((
                    PendingLod {
                        source_mesh: mesh.0.clone(),
                        top_uv: self.low_density_lod.0,
                        side_uv: self.low_density_lod.1,
                    },
                    material.clone(),
                    Transform::default(),
                    self.visibility_ranges[1].clone(),
                ));
            });
    }

    pub fn spawn_medium_density_building<R: RngExt>(
        &self,
        commands: &mut ChildSpawnerCommands,
        rng: &mut R,
        transform: Transform,
    ) {
        commands
            .spawn((transform, Visibility::default()))
            .with_children(|commands| {
                let (mesh, material) = self.medium_density.get_random_building(rng);
                commands.spawn((
                    mesh.clone(),
                    material.clone(),
                    Transform::default(),
                    self.visibility_ranges[0].clone(),
                ));
                commands.spawn((
                    PendingLod {
                        source_mesh: mesh.0.clone(),
                        top_uv: self.medium_density_lod.0,
                        side_uv: self.medium_density_lod.1,
                    },
                    material.clone(),
                    Transform::default(),
                    self.visibility_ranges[1].clone(),
                ));
            });
    }

    pub fn spawn_high_density_building<R: RngExt>(
        &self,
        commands: &mut ChildSpawnerCommands,
        rng: &mut R,
        transform: Transform,
    ) {
        commands
            .spawn((transform, Visibility::default()))
            .with_children(|commands| {
                let (mesh, material) = self.high_density.get_random_building(rng);
                commands.spawn((
                    mesh.clone(),
                    material.clone(),
                    Transform::default(),
                    self.visibility_ranges[0].clone(),
                ));
                commands.spawn((
                    PendingLod {
                        source_mesh: mesh.0.clone(),
                        top_uv: self.high_density_lod.0,
                        side_uv: self.high_density_lod.1,
                    },
                    material.clone(),
                    Transform::default(),
                    self.visibility_ranges[1].clone(),
                ));
            });
    }

    pub fn spawn_car<R: RngExt>(
        &self,
        commands: &mut ChildSpawnerCommands,
        rng: &mut R,
        transform: Transform,
        car: Car,
    ) {
        commands
            .spawn((transform, Visibility::default(), car))
            .with_children(|commands| {
                let car_scene = self.get_random_car(rng);
                commands.spawn((
                    SceneRoot(car_scene),
                    Transform::default(),
                    self.car_visibility_ranges[0].clone(),
                ));
                commands.spawn((
                    Mesh3d(self.car_lod.0.clone()),
                    MeshMaterial3d(self.car_lod.1.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                    self.car_visibility_ranges[1].clone(),
                ));
            });
    }
}

#[derive(Component)]
pub struct PendingLod {
    pub source_mesh: Handle<Mesh>,
    pub top_uv: Rect,
    pub side_uv: Rect,
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
    mut meshes: ResMut<Assets<Mesh>>,
    args: Res<Args>,
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

    let tree_small: Handle<Scene> =
        load_asset!(GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-suburban/tree-small.glb")));
    let tree_large: Handle<Scene> =
        load_asset!(GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-suburban/tree-large.glb")));

    let path_stones_long: Handle<Scene> = load_asset!(GltfAssetLabel::Scene(0)
        .from_asset(format!("{base_url}/city-kit-suburban/path-stones-long.glb")));
    let tree_lod_material =
        materials.add(StandardMaterial::from_color(Color::srgb_u8(90, 196, 135)));
    let tree_small_lod = {
        let mesh = meshes.add(Cuboid::new(0.1, 0.55, 0.1));
        (mesh, tree_lod_material.clone())
    };
    let tree_large_lod = {
        let mesh = meshes.add(Cuboid::new(0.2, 0.8, 0.2));
        (mesh, tree_lod_material.clone())
    };

    let fence: Handle<Scene> = load_asset!(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-suburban/fence.glb"))
    );

    let low_density_lod = (
        Rect::new(0.0, 1.0 - 0.75, 0.062, 1.0 - 0.5),
        Rect::new(0.375, 1.0 - 0.499, 0.437, 1.0 - 0.251),
    );
    let medium_density_lod = (
        Rect::new(0.626, 1.0 - 0.249, 0.687, 1.0 - 0.0),
        Rect::new(0.375, 1.0 - 0.499, 0.437, 1.0 - 0.251),
    );
    let high_density_lod = (
        Rect::new(0.626, 1.0 - 0.249, 0.687, 1.0 - 0.0),
        Rect::new(0.375, 1.0 - 0.499, 0.437, 1.0 - 0.251),
    );

    let car_lod = (
        // Once we merge the meshes of the car glb we'll be able to use the aabb instead of
        // manually created values
        meshes.add(Cuboid::new(1.0, 1.0, 2.5)),
        // We'll probably need to have specific uv for each cart type to have a color match.
        // This will be done once we have the merged meshes
        materials.add(StandardMaterial::default()),
    );

    commands.insert_resource(CityAssets {
        untyped_assets,
        cars,
        car_lod,
        crossroad,
        road_straight,
        high_density,
        medium_density,
        low_density,
        low_density_lod,
        medium_density_lod,
        high_density_lod,
        ground_tile,
        tree_small,
        tree_large,
        tree_small_lod,
        tree_large_lod,
        path_stones_long,
        fence,
        visibility_ranges: vec![
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: args.lod_min_range - 1.0..args.lod_min_range + 1.0,
                use_aabb: false,
            },
            VisibilityRange {
                start_margin: args.lod_min_range - 1.0..args.lod_min_range + 1.0,
                end_margin: args.lod_max_range..args.lod_max_range,
                use_aabb: false,
            },
        ],
        car_visibility_ranges: vec![
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: args.car_lod_min_range - 1.0..args.car_lod_min_range + 1.0,
                use_aabb: false,
            },
            VisibilityRange {
                start_margin: args.car_lod_min_range - 1.0..args.car_lod_min_range + 1.0,
                end_margin: args.car_lod_max_range..args.car_lod_max_range,
                use_aabb: false,
            },
        ],
    });
}

/// To get a cube that matches the Aabb we need to load the mesh data but that's not easy to do at
/// spawn time. So instead we can add the PendingLod component and it will generate a cube that
/// matches the aabb of the associated mesh.
///
/// The cube has different uvs for the top and the side and can be controlled too.
/// For buildings it's useful to have the roof color be the top of the cube and the sides to match.
/// This makes the lod transition a bit nicer.
pub fn resolve_pending_lods(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: Local<HashMap<Handle<Mesh>, Handle<Mesh>>>,
    pending: Query<(Entity, &PendingLod)>,
) {
    for (entity, pending) in &pending {
        let Some(source) = meshes.get(&pending.source_mesh) else {
            continue;
        };
        let Some(aabb) = source.compute_aabb() else {
            continue;
        };
        let lod_mesh = cache
            .entry(pending.source_mesh.clone())
            .or_insert_with(|| {
                let size = Vec3::from(aabb.half_extents) * 2.0;
                meshes.add(uv_cube(size, pending.top_uv, pending.side_uv))
            })
            .clone();
        let center = Vec3::from(aabb.center);
        commands
            .entity(entity)
            .insert((Mesh3d(lod_mesh), Transform::from_translation(center)))
            .remove::<PendingLod>();
    }
}

// Copied from bevy's default cube but with configurable uvs for the side and top
fn uv_cube(size: Vec3, top_uv: Rect, side_uv: Rect) -> Mesh {
    let min = -(size * 0.5);
    let max = size * 0.5;

    let top_uv_min_x = top_uv.min.x;
    let top_uv_min_y = top_uv.min.y;
    let top_uv_max_x = top_uv.max.x;
    let top_uv_max_y = top_uv.max.y;
    let side_uv_min_x = side_uv.min.x;
    let side_uv_min_y = side_uv.min.y;
    let side_uv_max_x = side_uv.max.x;
    let side_uv_max_y = side_uv.max.y;

    #[rustfmt::skip]
    let vertices = &[
        // Front
        ([min.x, min.y, max.z], [0.0, 0.0,  1.0], [side_uv_min_x, side_uv_min_y]),
        ([max.x, min.y, max.z], [0.0, 0.0,  1.0], [side_uv_max_x, side_uv_min_y]),
        ([max.x, max.y, max.z], [0.0, 0.0,  1.0], [side_uv_max_x, side_uv_max_y]),
        ([min.x, max.y, max.z], [0.0, 0.0,  1.0], [side_uv_min_x, side_uv_max_y]),
        // Back
        ([min.x, max.y, min.z], [0.0, 0.0, -1.0], [side_uv_max_x, side_uv_min_y]),
        ([max.x, max.y, min.z], [0.0, 0.0, -1.0], [side_uv_min_x, side_uv_min_y]),
        ([max.x, min.y, min.z], [0.0, 0.0, -1.0], [side_uv_min_x, side_uv_max_y]),
        ([min.x, min.y, min.z], [0.0, 0.0, -1.0], [side_uv_max_x, side_uv_max_y]),
        // Right
        ([max.x, min.y, min.z], [ 1.0, 0.0, 0.0], [side_uv_min_x, side_uv_min_y]),
        ([max.x, max.y, min.z], [ 1.0, 0.0, 0.0], [side_uv_max_x, side_uv_min_y]),
        ([max.x, max.y, max.z], [ 1.0, 0.0, 0.0], [side_uv_max_x, side_uv_max_y]),
        ([max.x, min.y, max.z], [ 1.0, 0.0, 0.0], [side_uv_min_x, side_uv_max_y]),
        // Left
        ([min.x, min.y, max.z], [-1.0, 0.0, 0.0], [side_uv_max_x, side_uv_min_y]),
        ([min.x, max.y, max.z], [-1.0, 0.0, 0.0], [side_uv_min_x, side_uv_min_y]),
        ([min.x, max.y, min.z], [-1.0, 0.0, 0.0], [side_uv_min_x, side_uv_max_y]),
        ([min.x, min.y, min.z], [-1.0, 0.0, 0.0], [side_uv_max_x, side_uv_max_y]),
        // Top
        ([max.x, max.y, min.z], [0.0, 1.0, 0.0], [top_uv_max_x, top_uv_min_y]),
        ([min.x, max.y, min.z], [0.0, 1.0, 0.0], [top_uv_min_x, top_uv_min_y]),
        ([min.x, max.y, max.z], [0.0, 1.0, 0.0], [top_uv_min_x, top_uv_max_y]),
        ([max.x, max.y, max.z], [0.0, 1.0, 0.0], [top_uv_max_x, top_uv_max_y]),
        // Bottom
        ([max.x, min.y, max.z], [0.0, -1.0, 0.0], [side_uv_min_x, side_uv_min_y]),
        ([min.x, min.y, max.z], [0.0, -1.0, 0.0], [side_uv_max_x, side_uv_min_y]),
        ([min.x, min.y, min.z], [0.0, -1.0, 0.0], [side_uv_max_x, side_uv_max_y]),
        ([max.x, min.y, min.z], [0.0, -1.0, 0.0], [side_uv_min_x, side_uv_max_y]),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

    let indices = Indices::U32(vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ]);

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(indices)
}
