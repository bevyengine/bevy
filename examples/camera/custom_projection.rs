//! Demonstrates how to define and use custom camera projections.

use bevy::camera::CameraProjection;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Like a perspective projection, but the vanishing point is not centered.
#[derive(Debug, Clone)]
struct ObliquePerspectiveProjection {
    horizontal_obliqueness: f32,
    vertical_obliqueness: f32,
    perspective: PerspectiveProjection,
}

/// Implement the [`CameraProjection`] trait for our custom projection:
impl CameraProjection for ObliquePerspectiveProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        let mut mat = self.perspective.get_clip_from_view();
        mat.col_mut(2)[0] = self.horizontal_obliqueness;
        mat.col_mut(2)[1] = self.vertical_obliqueness;
        mat
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &bevy::camera::SubCameraView) -> Mat4 {
        let mut mat = self.perspective.get_clip_from_view_for_sub(sub_view);
        mat.col_mut(2)[0] = self.horizontal_obliqueness;
        mat.col_mut(2)[1] = self.vertical_obliqueness;
        mat
    }

    fn update(&mut self, width: f32, height: f32) {
        self.perspective.update(width, height);
    }

    fn far(&self) -> f32 {
        self.perspective.far
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        self.perspective.get_frustum_corners(z_near, z_far)
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        // Use our custom projection:
        Projection::custom(ObliquePerspectiveProjection {
            horizontal_obliqueness: 0.2,
            vertical_obliqueness: 0.6,
            perspective: PerspectiveProjection::default(),
        }),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Scene setup
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}
