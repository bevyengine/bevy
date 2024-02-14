//! Demonstrates UV mappings of the [`CircularSector`] and [`CircularSegment`] primitives.

use std::f32::consts::PI;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_internal::render::mesh::{
    CircularSectorMeshBuilder, CircularSegmentMeshBuilder, CircularShapeUvMode,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let material = materials.add(asset_server.load("branding/icon.png"));

    commands.spawn(Camera2dBundle {
        camera: Camera {
            clear_color: ClearColorConfig::Custom(Color::DARK_GRAY),
            ..default()
        },
        ..default()
    });

    const UPPER_Y: f32 = 50.0;
    const LOWER_Y: f32 = -50.0;
    const FIRST_X: f32 = -450.0;
    const OFFSET: f32 = 100.0;
    const NUM_SLICES: i32 = 8;

    // This draws NUM_SLICES copies of the Bevy logo as circular sectors and segments,
    // with successively larger angles up to a complete circle.
    for i in 0..NUM_SLICES {
        let fraction = (i + 1) as f32 / NUM_SLICES as f32;

        let sector = CircularSector::from_fraction(40.0, fraction);
        // We want to rotate the circular sector so that the sectors appear clockwise from north.
        // We must rotate it both in the Transform and in the mesh's UV mappings.
        let sector_angle = -sector.half_angle();
        let sector_mesh =
            CircularSectorMeshBuilder::new(sector).uv_mode(CircularShapeUvMode::Mask {
                angle: sector_angle,
            });
        commands.spawn(MaterialMesh2dBundle {
            mesh: meshes.add(sector_mesh).into(),
            material: material.clone(),
            transform: Transform {
                translation: Vec3::new(FIRST_X + OFFSET * i as f32, 2.0 * UPPER_Y, 0.0),
                rotation: Quat::from_rotation_z(sector_angle),
                ..default()
            },
            ..default()
        });

        let segment = CircularSegment::from_fraction(40.0, fraction);
        // For the circular segment, we will draw Bevy charging forward, which requires rotating the
        // shape and texture by 90 degrees.
        //
        // Note that this may be unintuitive; it may feel like we should rotate the texture by the
        // opposite angle to preserve the orientation of Bevy. But the angle is not the angle of the
        // texture itself, rather it is the angle at which the vertices are mapped onto the texture.
        // so it is the negative of what you might otherwise expect.
        let segment_angle = -PI / 2.0;
        let segment_mesh =
            CircularSegmentMeshBuilder::new(segment).uv_mode(CircularShapeUvMode::Mask {
                angle: -segment_angle,
            });
        commands.spawn(MaterialMesh2dBundle {
            mesh: meshes.add(segment_mesh).into(),
            material: material.clone(),
            transform: Transform {
                translation: Vec3::new(FIRST_X + OFFSET * i as f32, LOWER_Y, 0.0),
                rotation: Quat::from_rotation_z(segment_angle),
                ..default()
            },
            ..default()
        });
    }
}
