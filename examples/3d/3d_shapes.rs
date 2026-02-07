//! Here we use shape primitives to generate meshes for 3d objects as well as attaching a runtime-generated patterned texture to each 3d object.
//!
//! "Shape primitives" here are just the mathematical definition of certain shapes, they're not meshes on their own! A sphere with radius `1.0` can be defined with [`Sphere::new(1.0)`][Sphere::new] but all this does is store the radius. So we need to turn these descriptions of shapes into meshes.
//!
//! While a shape is not a mesh, turning it into one in Bevy is easy. In this example we call [`asset_commands.spawn_asset(/* Shape here! */)`] and `.into()`. There's an implementation for [`From`] on shape primitives into [`Mesh`], and since we are spawning [`Mesh3d`] components, Rust knows to convert our shapes to [`Mesh`]es.
//!
//! [`Extrusion`] lets us turn 2D shape primitives into versions of those shapes that have volume by extruding them. A 1x1 square that gets wrapped in this with an extrusion depth of 2 will give us a rectangular prism of size 1x1x2, but here we're just extruding these 2d shapes by depth 1.
//!
//! The material applied to these shapes is a texture that we generate at run time by looping through a "palette" of RGBA values (stored adjacent to each other in the array) and writing values to positions in another array that represents the buffer for an 8x8 texture. This texture is then registered with the assets system just one time, with that [`Handle<StandardMaterial>`] then applied to all the shapes in this example.
//!
//! The mesh and material are [`Handle<Mesh>`] and [`Handle<StandardMaterial>`] at the moment, neither of which implement `Component` on their own. Handles are put behind "newtypes" to prevent ambiguity, as some entities might want to have handles to meshes (or images, or materials etc.) for different purposes! All we need to do to make them rendering-relevant components is wrap the mesh handle and the material handle in [`Mesh3d`] and [`MeshMaterial3d`] respectively.
//!
//! You can toggle wireframes with the space bar except on wasm. Wasm does not support
//! `POLYGON_MODE_LINE` on the gpu.

use std::f32::consts::PI;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::basic::SILVER,
    input::common_conditions::{input_just_pressed, input_toggle_active},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            #[cfg(not(target_arch = "wasm32"))]
            WireframePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                rotate.run_if(input_toggle_active(true, KeyCode::KeyR)),
                advance_rows.run_if(input_just_pressed(KeyCode::Tab)),
                #[cfg(not(target_arch = "wasm32"))]
                toggle_wireframe,
            ),
        )
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 14.0;
const Z_EXTENT: f32 = 8.0;
const THICKNESS: f32 = 0.1;

fn setup(mut commands: Commands, mut asset_commands: AssetCommands) {
    let debug_texture = asset_commands.spawn_asset(uv_debug_texture());
    let debug_material = asset_commands.spawn_asset(StandardMaterial {
        base_color_texture: Some(debug_texture),
        ..default()
    });

    let shapes = [
        asset_commands.spawn_asset(Cuboid::default().into()),
        asset_commands.spawn_asset(Tetrahedron::default().into()),
        asset_commands.spawn_asset(Capsule3d::default().into()),
        asset_commands.spawn_asset(Torus::default().into()),
        asset_commands.spawn_asset(Cylinder::default().into()),
        asset_commands.spawn_asset(Cone::default().into()),
        asset_commands.spawn_asset(ConicalFrustum::default().into()),
        asset_commands.spawn_asset(Sphere::default().mesh().ico(5).unwrap()),
        asset_commands.spawn_asset(Sphere::default().mesh().uv(32, 18)),
        asset_commands.spawn_asset(Segment3d::default().into()),
        asset_commands.spawn_asset(
            Polyline3d::new(vec![
                Vec3::new(-0.5, 0.0, 0.0),
                Vec3::new(0.5, 0.0, 0.0),
                Vec3::new(0.0, 0.5, 0.0),
            ])
            .into(),
        ),
    ];

    let extrusions = [
        asset_commands.spawn_asset(Extrusion::new(Rectangle::default(), 1.).into()),
        asset_commands.spawn_asset(Extrusion::new(Capsule2d::default(), 1.).into()),
        asset_commands.spawn_asset(Extrusion::new(Annulus::default(), 1.).into()),
        asset_commands.spawn_asset(Extrusion::new(Circle::default(), 1.).into()),
        asset_commands.spawn_asset(Extrusion::new(Ellipse::default(), 1.).into()),
        asset_commands.spawn_asset(Extrusion::new(RegularPolygon::default(), 1.).into()),
        asset_commands.spawn_asset(Extrusion::new(Triangle2d::default(), 1.).into()),
    ];

    let ring_extrusions = [
        asset_commands
            .spawn_asset(Extrusion::new(Rectangle::default().to_ring(THICKNESS), 1.).into()),
        asset_commands
            .spawn_asset(Extrusion::new(Capsule2d::default().to_ring(THICKNESS), 1.).into()),
        asset_commands
            .spawn_asset(Extrusion::new(Ring::new(Circle::new(1.0), Circle::new(0.5)), 1.).into()),
        asset_commands.spawn_asset(Extrusion::new(Circle::default().to_ring(THICKNESS), 1.).into()),
        asset_commands.spawn_asset(
            Extrusion::new(
                {
                    // This is an approximation; Ellipse does not implement Inset as concentric ellipses do not have parallel curves
                    let outer = Ellipse::default();
                    let mut inner = outer;
                    inner.half_size -= Vec2::splat(THICKNESS);
                    Ring::new(outer, inner)
                },
                1.,
            )
            .into(),
        ),
        asset_commands
            .spawn_asset(Extrusion::new(RegularPolygon::default().to_ring(THICKNESS), 1.).into()),
        asset_commands
            .spawn_asset(Extrusion::new(Triangle2d::default().to_ring(THICKNESS), 1.).into()),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                2.0,
                Row::Front.z(),
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
            Row::Front,
        ));
    }

    let num_extrusions = extrusions.len();

    for (i, shape) in extrusions.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                -EXTRUSION_X_EXTENT / 2.
                    + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                2.0,
                Row::Middle.z(),
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
            Row::Middle,
        ));
    }

    let num_ring_extrusions = ring_extrusions.len();

    for (i, shape) in ring_extrusions.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                -EXTRUSION_X_EXTENT / 2.
                    + i as f32 / (num_ring_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                2.0,
                Row::Rear.z(),
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
            Row::Rear,
        ));
    }

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // ground plane
    commands.spawn((
        Mesh3d(
            asset_commands.spawn_asset(
                Plane3d::default()
                    .mesh()
                    .size(50.0, 50.0)
                    .subdivisions(10)
                    .into(),
            ),
        ),
        MeshMaterial3d(asset_commands.spawn_asset(StandardMaterial::from(Color::from(SILVER)))),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    let mut text = "\
        Press 'R' to pause/resume rotation\n\
        Press 'Tab' to cycle through rows"
        .to_string();
    #[cfg(not(target_arch = "wasm32"))]
    text.push_str("\nPress 'Space' to toggle wireframes");

    commands.spawn((
        Text::new(text),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}

#[derive(Component, Clone, Copy)]
enum Row {
    Front,
    Middle,
    Rear,
}

impl Row {
    fn z(self) -> f32 {
        match self {
            Row::Front => Z_EXTENT / 2.,
            Row::Middle => 0.,
            Row::Rear => -Z_EXTENT / 2.,
        }
    }

    fn advance(self) -> Self {
        match self {
            Row::Front => Row::Rear,
            Row::Middle => Row::Front,
            Row::Rear => Row::Middle,
        }
    }
}

fn advance_rows(mut shapes: Query<(&mut Row, &mut Transform), With<Shape>>) {
    for (mut row, mut transform) in &mut shapes {
        *row = row.advance();
        transform.translation.z = row.z();
    }
}
