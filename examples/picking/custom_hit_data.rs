//! Demonstrates a custom picking backend with custom hit data.
//!
//! The example contains pickable 3D meshes. When a mesh is hovered, a custom
//! picking backend performs a ray cast against the mesh and retrieves the
//! triangle that was hit. The triangle vertices are stored in a custom struct
//! (`TriangleHitInfo`) that implements `HitDataExtra`, and saved into `HitData`
//! structs. This information is not available by default in `HitData` and thus
//! requires its `extra` field. A follow-up system reads the hit data and draws
//! an outline around the hovered triangle using gizmos.

use bevy::{
    color::palettes::css::*,
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        mesh_picking::{
            ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility},
            MeshPickingSettings,
        },
        prelude::*,
        PickingSettings, PickingSystems,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .insert_resource(MeshPickingSettings {
            require_markers: true,
            ..default()
        })
        .insert_resource(PickingSettings {
            is_window_picking_enabled: false,
            ..default()
        })
        .init_resource::<HoveredTriangles>()
        .add_systems(Startup, (setup_gizmos, setup_scene))
        .add_systems(
            PreUpdate,
            (
                custom_backend_system.in_set(PickingSystems::Backend),
                cache_hovered_triangles.after(PickingSystems::Backend),
            ),
        )
        .add_systems(Update, draw_hit_gizmos)
        .run();
}

/// The custom hit data used by our picking backend. All structs that implement
/// `Send + Sync + fmt::Debug + 'static` automatically implement `HitDataExtra`
/// and can be used as extra data in `HitData`.
#[derive(Debug)]
struct TriangleHitInfo {
    triangle_vertices: Option<[Vec3; 3]>,
}

#[derive(Resource, Default)]
struct HoveredTriangles(Vec<TriangleOverlay>);

struct TriangleOverlay {
    position: Vec3,
    normal: Vec3,
    vertices: [Vec3; 3],
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shapes: [(Mesh, Color); 3] = [
        (Cuboid::default().into(), RED.into()),
        (Sphere::default().mesh().ico(2).unwrap(), GREEN.into()),
        (Cylinder::default().into(), BLUE.into()),
    ];

    for (i, (mesh, color)) in shapes.iter().enumerate() {
        let x = i as f32 * 1.5 - 1.5;
        let material = materials.add(StandardMaterial::from_color(*color));

        commands.spawn((
            Mesh3d(meshes.add(mesh.clone())),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0.5, 0.0),
            Pickable::default(),
        ));
    }

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 30.0))),
        MeshMaterial3d(materials.add(Color::from(DARK_GRAY))),
        Pickable::IGNORE,
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(0.0, 8.0, 4.0)));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.5, 6.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
    ));
}

fn setup_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
    config.line.width = 3.0;
}

fn custom_backend_system(
    ray_map: Res<RayMap>,
    cameras: Query<&Camera>,
    pickables: Query<&Pickable>,
    mut ray_cast: MeshRayCast,
    mut pointer_hits: MessageWriter<PointerHits>,
) {
    for (&ray_id, &ray) in ray_map.iter() {
        let Ok(camera) = cameras.get(ray_id.camera) else {
            continue;
        };

        let settings = MeshRayCastSettings {
            visibility: RayCastVisibility::VisibleInView,
            filter: &|e| pickables.get(e).is_ok_and(|p| p.is_hoverable),
            early_exit_test: &|entity_hit| {
                pickables
                    .get(entity_hit)
                    .is_ok_and(|p| p.should_block_lower)
            },
        };

        let picks: Vec<(Entity, HitData)> = ray_cast
            .cast_ray(ray, &settings)
            .iter()
            .map(|(entity, hit)| {
                let extra = TriangleHitInfo {
                    triangle_vertices: hit.triangle,
                };

                let hit_data = HitData::new_with_extra(
                    ray_id.camera,
                    hit.distance,
                    Some(hit.point),
                    Some(hit.normal),
                    extra,
                );

                (*entity, hit_data)
            })
            .collect();

        if !picks.is_empty() {
            pointer_hits.write(PointerHits::new(ray_id.pointer, picks, camera.order as f32));
        }
    }
}

fn cache_hovered_triangles(
    mut pointer_hits: MessageReader<PointerHits>,
    mut hovered_triangles: ResMut<HoveredTriangles>,
) {
    hovered_triangles.0.clear();

    for hits in pointer_hits.read() {
        for (_, hit) in &hits.picks {
            let (Some(position), Some(normal)) = (hit.position, hit.normal) else {
                continue;
            };

            let Some(info) = hit.extra_as::<TriangleHitInfo>() else {
                continue;
            };
            let Some(vertices) = info.triangle_vertices else {
                continue;
            };

            hovered_triangles.0.push(TriangleOverlay {
                position,
                normal,
                vertices,
            });
        }
    }
}

fn draw_hit_gizmos(hovered_triangles: Res<HoveredTriangles>, mut gizmos: Gizmos) {
    for triangle in &hovered_triangles.0 {
        gizmos.arrow(
            triangle.position,
            triangle.position + triangle.normal.normalize() * 0.5,
            WHITE,
        );

        let vertices = triangle.vertices;
        let center = (vertices[0] + vertices[1] + vertices[2]) / 3.0;
        let offset = triangle.normal.normalize_or_zero() * 0.025;

        // The outline is made bigger and offset a bit to prevent being covered
        // by the mesh
        let outline = vertices.map(|vertex| center + (vertex - center) * 1.05 + offset);

        gizmos.line(outline[0], outline[1], WHITE);
        gizmos.line(outline[1], outline[2], WHITE);
        gizmos.line(outline[2], outline[0], WHITE);
    }
}
