//! Procedural geometry generation for machines.
//!
//! These are fallback visuals used when GLTF models aren't available.
//! They provide recognizable shapes for each machine type using primitive geometry.

use bevy_asset::Assets;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_math::{primitives::*, Quat, Vec3};
use bevy_mesh::{Mesh, Mesh3d};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_render::alpha::AlphaMode;
use bevy_transform::components::Transform;

use crate::machines::BladeVisual;

/// Marker component for procedurally generated machine visuals.
#[derive(Component, Default)]
pub struct ProceduralMachineVisual;

/// Spawns procedural geometry for a bulldozer as children of the given entity.
pub fn spawn_procedural_dozer(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
) {
    // Colors - CAT yellow theme
    let yellow = Color::srgb(0.95, 0.75, 0.1);
    let dark_yellow = Color::srgb(0.8, 0.6, 0.05);
    let black = Color::srgb(0.1, 0.1, 0.1);
    let dark_gray = Color::srgb(0.2, 0.2, 0.2);
    let glass = Color::srgba(0.3, 0.4, 0.5, 0.6);

    // Materials
    let body_mat = materials.add(StandardMaterial {
        base_color: yellow,
        metallic: 0.2,
        perceptual_roughness: 0.7,
        ..Default::default()
    });
    let accent_mat = materials.add(StandardMaterial {
        base_color: dark_yellow,
        metallic: 0.3,
        perceptual_roughness: 0.6,
        ..Default::default()
    });
    let track_mat = materials.add(StandardMaterial {
        base_color: black,
        metallic: 0.1,
        perceptual_roughness: 0.9,
        ..Default::default()
    });
    let metal_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..Default::default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: glass,
        metallic: 0.0,
        perceptual_roughness: 0.1,
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });
    let blade_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..Default::default()
    });

    // Meshes
    let track_mesh = meshes.add(Cuboid::new(1.0, 0.8, 4.0));
    let body_mesh = meshes.add(Cuboid::new(2.4, 1.2, 3.2));
    let cab_mesh = meshes.add(Cuboid::new(1.8, 1.4, 1.6));
    let roof_mesh = meshes.add(Cuboid::new(2.0, 0.15, 1.8));
    let engine_mesh = meshes.add(Cuboid::new(2.2, 0.8, 1.2));
    let exhaust_mesh = meshes.add(Cylinder::new(0.08, 0.6));
    let blade_mesh = meshes.add(Cuboid::new(4.0, 1.2, 0.25));
    let blade_edge_mesh = meshes.add(Cuboid::new(4.2, 0.15, 0.3));
    let arm_mesh = meshes.add(Cuboid::new(0.15, 0.15, 1.5));
    let ripper_mesh = meshes.add(Cuboid::new(0.8, 0.4, 0.15));
    let ripper_tooth_mesh = meshes.add(Cuboid::new(0.1, 0.6, 0.1));
    let window_mesh = meshes.add(Cuboid::new(1.6, 0.8, 0.05));

    commands.entity(parent).with_children(|parent| {
        // Marker for procedural visual
        parent.spawn(ProceduralMachineVisual);

        // Left track
        parent.spawn((
            Mesh3d(track_mesh.clone()),
            MeshMaterial3d(track_mat.clone()),
            Transform::from_xyz(-1.0, 0.0, 0.0),
        ));

        // Right track
        parent.spawn((
            Mesh3d(track_mesh.clone()),
            MeshMaterial3d(track_mat.clone()),
            Transform::from_xyz(1.0, 0.0, 0.0),
        ));

        // Main body
        parent.spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 0.9, 0.2),
        ));

        // Cab
        parent.spawn((
            Mesh3d(cab_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 2.0, 0.5),
        ));

        // Cab roof
        parent.spawn((
            Mesh3d(roof_mesh),
            MeshMaterial3d(accent_mat.clone()),
            Transform::from_xyz(0.0, 2.8, 0.5),
        ));

        // Cab windows (front)
        parent.spawn((
            Mesh3d(window_mesh),
            MeshMaterial3d(glass_mat.clone()),
            Transform::from_xyz(0.0, 2.1, -0.35),
        ));

        // Engine hood
        parent.spawn((
            Mesh3d(engine_mesh),
            MeshMaterial3d(accent_mat.clone()),
            Transform::from_xyz(0.0, 1.0, -1.2),
        ));

        // Exhaust stack
        parent.spawn((
            Mesh3d(exhaust_mesh),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_xyz(0.8, 1.8, -0.8),
        ));

        // Blade assembly
        parent
            .spawn((
                Transform::from_xyz(0.0, 0.3, -2.8),
                BladeVisual,
            ))
            .with_children(|blade_parent| {
                // Main blade
                blade_parent.spawn((
                    Mesh3d(blade_mesh),
                    MeshMaterial3d(blade_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                ));

                // Cutting edge
                blade_parent.spawn((
                    Mesh3d(blade_edge_mesh),
                    MeshMaterial3d(metal_mat.clone()),
                    Transform::from_xyz(0.0, -0.6, 0.0),
                ));

                // Left push arm
                blade_parent.spawn((
                    Mesh3d(arm_mesh.clone()),
                    MeshMaterial3d(body_mat.clone()),
                    Transform::from_xyz(-1.5, 0.3, 0.8),
                ));

                // Right push arm
                blade_parent.spawn((
                    Mesh3d(arm_mesh.clone()),
                    MeshMaterial3d(body_mat.clone()),
                    Transform::from_xyz(1.5, 0.3, 0.8),
                ));
            });

        // Rear ripper
        parent.spawn((
            Mesh3d(ripper_mesh),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_xyz(0.0, 0.4, 2.5),
        ));

        // Ripper teeth
        for i in -1..=1 {
            parent.spawn((
                Mesh3d(ripper_tooth_mesh.clone()),
                MeshMaterial3d(metal_mat.clone()),
                Transform::from_xyz(i as f32 * 0.25, 0.0, 2.5),
            ));
        }
    });
}

/// Spawns procedural geometry for an excavator as children of the given entity.
pub fn spawn_procedural_excavator(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
) {
    let yellow = Color::srgb(0.95, 0.75, 0.1);
    let black = Color::srgb(0.1, 0.1, 0.1);
    let dark_gray = Color::srgb(0.25, 0.25, 0.25);

    let body_mat = materials.add(StandardMaterial {
        base_color: yellow,
        metallic: 0.2,
        perceptual_roughness: 0.7,
        ..Default::default()
    });
    let track_mat = materials.add(StandardMaterial {
        base_color: black,
        metallic: 0.1,
        perceptual_roughness: 0.9,
        ..Default::default()
    });
    let metal_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..Default::default()
    });

    commands.entity(parent).with_children(|parent| {
        // Marker for procedural visual
        parent.spawn(ProceduralMachineVisual);

        // Track assembly (wider than dozer)
        let track_mesh = meshes.add(Cuboid::new(1.2, 0.9, 4.5));
        parent.spawn((
            Mesh3d(track_mesh.clone()),
            MeshMaterial3d(track_mat.clone()),
            Transform::from_xyz(-1.5, 0.0, 0.0),
        ));
        parent.spawn((
            Mesh3d(track_mesh),
            MeshMaterial3d(track_mat),
            Transform::from_xyz(1.5, 0.0, 0.0),
        ));

        // Upper body (rotates)
        let body_mesh = meshes.add(Cuboid::new(2.8, 1.5, 3.5));
        parent.spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 1.2, 0.0),
        ));

        // Cab
        let cab_mesh = meshes.add(Cuboid::new(1.8, 1.6, 1.5));
        parent.spawn((
            Mesh3d(cab_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(-0.3, 2.5, -0.8),
        ));

        // Boom (main arm)
        let boom_mesh = meshes.add(Cuboid::new(0.5, 0.6, 4.0));
        parent.spawn((
            Mesh3d(boom_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 2.5, -3.5).with_rotation(Quat::from_rotation_x(-0.4)),
        ));

        // Stick (second arm segment)
        let stick_mesh = meshes.add(Cuboid::new(0.4, 0.5, 3.0));
        parent.spawn((
            Mesh3d(stick_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 1.5, -6.5).with_rotation(Quat::from_rotation_x(0.6)),
        ));

        // Bucket
        let bucket_mesh = meshes.add(Cuboid::new(1.2, 0.8, 0.8));
        parent.spawn((
            Mesh3d(bucket_mesh),
            MeshMaterial3d(metal_mat),
            Transform::from_xyz(0.0, -0.5, -7.5),
        ));

        // Counterweight
        let counterweight_mesh = meshes.add(Cuboid::new(2.4, 1.0, 1.2));
        parent.spawn((
            Mesh3d(counterweight_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_xyz(0.0, 1.5, 2.0),
        ));
    });
}

/// Spawns procedural geometry for a wheel loader as children of the given entity.
pub fn spawn_procedural_loader(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
) {
    let yellow = Color::srgb(0.95, 0.75, 0.1);
    let black = Color::srgb(0.15, 0.15, 0.15);
    let dark_gray = Color::srgb(0.3, 0.3, 0.3);

    let body_mat = materials.add(StandardMaterial {
        base_color: yellow,
        metallic: 0.2,
        perceptual_roughness: 0.7,
        ..Default::default()
    });
    let tire_mat = materials.add(StandardMaterial {
        base_color: black,
        metallic: 0.0,
        perceptual_roughness: 0.95,
        ..Default::default()
    });
    let bucket_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.6,
        perceptual_roughness: 0.4,
        ..Default::default()
    });

    commands.entity(parent).with_children(|parent| {
        parent.spawn(ProceduralMachineVisual);

        // Large wheels (4)
        let wheel_mesh = meshes.add(Cylinder::new(0.8, 0.6));
        for (x, z) in [(-1.2, -1.5), (1.2, -1.5), (-1.2, 1.5), (1.2, 1.5)] {
            parent.spawn((
                Mesh3d(wheel_mesh.clone()),
                MeshMaterial3d(tire_mat.clone()),
                Transform::from_xyz(x, 0.8, z).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
        }

        // Main body
        let body_mesh = meshes.add(Cuboid::new(2.0, 1.8, 3.5));
        parent.spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 1.8, 0.5),
        ));

        // Cab
        let cab_mesh = meshes.add(Cuboid::new(1.6, 1.4, 1.4));
        parent.spawn((
            Mesh3d(cab_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 3.2, 0.8),
        ));

        // Lift arms
        let arm_mesh = meshes.add(Cuboid::new(0.2, 0.3, 3.0));
        parent.spawn((
            Mesh3d(arm_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(-0.9, 2.5, -1.5).with_rotation(Quat::from_rotation_x(-0.3)),
        ));
        parent.spawn((
            Mesh3d(arm_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_xyz(0.9, 2.5, -1.5).with_rotation(Quat::from_rotation_x(-0.3)),
        ));

        // Bucket
        let bucket_mesh = meshes.add(Cuboid::new(2.4, 1.0, 1.2));
        parent.spawn((
            Mesh3d(bucket_mesh),
            MeshMaterial3d(bucket_mat),
            Transform::from_xyz(0.0, 1.5, -3.5),
        ));
    });
}

/// Spawns procedural geometry for a dump truck as children of the given entity.
pub fn spawn_procedural_dump_truck(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
) {
    let yellow = Color::srgb(0.95, 0.75, 0.1);
    let black = Color::srgb(0.15, 0.15, 0.15);
    let dark_gray = Color::srgb(0.35, 0.35, 0.35);

    let body_mat = materials.add(StandardMaterial {
        base_color: yellow,
        metallic: 0.2,
        perceptual_roughness: 0.7,
        ..Default::default()
    });
    let tire_mat = materials.add(StandardMaterial {
        base_color: black,
        metallic: 0.0,
        perceptual_roughness: 0.95,
        ..Default::default()
    });
    let bed_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.5,
        perceptual_roughness: 0.5,
        ..Default::default()
    });

    commands.entity(parent).with_children(|parent| {
        parent.spawn(ProceduralMachineVisual);

        // Massive wheels (6 - dual rear axle)
        let wheel_mesh = meshes.add(Cylinder::new(1.2, 0.8));
        // Front wheels
        for x in [-1.8, 1.8] {
            parent.spawn((
                Mesh3d(wheel_mesh.clone()),
                MeshMaterial3d(tire_mat.clone()),
                Transform::from_xyz(x, 1.2, -3.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
        }
        // Rear wheels (dual)
        for x in [-1.6, -2.2, 1.6, 2.2] {
            parent.spawn((
                Mesh3d(wheel_mesh.clone()),
                MeshMaterial3d(tire_mat.clone()),
                Transform::from_xyz(x, 1.2, 2.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
        }

        // Cab
        let cab_mesh = meshes.add(Cuboid::new(2.8, 2.2, 2.0));
        parent.spawn((
            Mesh3d(cab_mesh),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_xyz(0.0, 3.0, -3.5),
        ));

        // Dump bed
        let bed_mesh = meshes.add(Cuboid::new(3.2, 2.0, 6.0));
        parent.spawn((
            Mesh3d(bed_mesh),
            MeshMaterial3d(bed_mat),
            Transform::from_xyz(0.0, 3.5, 1.5),
        ));

        // Engine hood
        let hood_mesh = meshes.add(Cuboid::new(2.4, 1.2, 1.5));
        parent.spawn((
            Mesh3d(hood_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_xyz(0.0, 2.0, -5.0),
        ));
    });
}
