//! Demonstrates UV mappings of the [`CircularSector`] and [`CircularSegment`] primitives.
//!
//! Also draws the bounding boxes and circles of the primitives.

use std::f32::consts::FRAC_PI_2;

use bevy::{
    color::palettes::css::{BLUE, GRAY, RED},
    math::{
        bounding::{Bounded2d, BoundingVolume},
        Isometry2d,
    },
    mesh::{CircularMeshUvMode, CircularSectorMeshBuilder, CircularSegmentMeshBuilder},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                draw_bounds::<CircularSector>,
                draw_bounds::<CircularSegment>,
            ),
        )
        .run();
}

#[derive(Component, Debug)]
struct DrawBounds<Shape: Bounded2d + Send + Sync + 'static>(Shape);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let material = materials.add(asset_server.load("branding/icon.png"));

    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(GRAY.into()),
            ..default()
        },
    ));

    const NUM_SLICES: i32 = 8;
    const SPACING_X: f32 = 100.0;
    const OFFSET_X: f32 = SPACING_X * (NUM_SLICES - 1) as f32 / 2.0;

    // This draws NUM_SLICES copies of the Bevy logo as circular sectors and segments,
    // with successively larger angles up to a complete circle.
    for i in 0..NUM_SLICES {
        let fraction = (i + 1) as f32 / NUM_SLICES as f32;

        let sector = CircularSector::from_turns(40.0, fraction);
        // We want to rotate the circular sector so that the sectors appear clockwise from north.
        // We must rotate it both in the Transform and in the mesh's UV mappings.
        let sector_angle = -sector.half_angle();
        let sector_mesh =
            CircularSectorMeshBuilder::new(sector).uv_mode(CircularMeshUvMode::Mask {
                angle: sector_angle,
            });
        commands.spawn((
            Mesh2d(meshes.add(sector_mesh)),
            MeshMaterial2d(material.clone()),
            Transform {
                translation: Vec3::new(SPACING_X * i as f32 - OFFSET_X, 50.0, 0.0),
                rotation: Quat::from_rotation_z(sector_angle),
                ..default()
            },
            DrawBounds(sector),
        ));

        let segment = CircularSegment::from_turns(40.0, fraction);
        // For the circular segment, we will draw Bevy charging forward, which requires rotating the
        // shape and texture by 90 degrees.
        //
        // Note that this may be unintuitive; it may feel like we should rotate the texture by the
        // opposite angle to preserve the orientation of Bevy. But the angle is not the angle of the
        // texture itself, rather it is the angle at which the vertices are mapped onto the texture.
        // so it is the negative of what you might otherwise expect.
        let segment_angle = -FRAC_PI_2;
        let segment_mesh =
            CircularSegmentMeshBuilder::new(segment).uv_mode(CircularMeshUvMode::Mask {
                angle: -segment_angle,
            });
        commands.spawn((
            Mesh2d(meshes.add(segment_mesh)),
            MeshMaterial2d(material.clone()),
            Transform {
                translation: Vec3::new(SPACING_X * i as f32 - OFFSET_X, -50.0, 0.0),
                rotation: Quat::from_rotation_z(segment_angle),
                ..default()
            },
            DrawBounds(segment),
        ));
    }
}

fn draw_bounds<Shape: Bounded2d + Send + Sync + 'static>(
    q: Query<(&DrawBounds<Shape>, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    for (shape, transform) in &q {
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        let translation = translation.truncate();
        let rotation = rotation.to_euler(EulerRot::XYZ).2;
        let isometry = Isometry2d::new(translation, Rot2::radians(rotation));

        let aabb = shape.0.aabb_2d(isometry);
        gizmos.rect_2d(aabb.center(), aabb.half_size() * 2.0, RED);

        let bounding_circle = shape.0.bounding_circle(isometry);
        gizmos.circle_2d(bounding_circle.center, bounding_circle.radius(), BLUE);
    }
}
