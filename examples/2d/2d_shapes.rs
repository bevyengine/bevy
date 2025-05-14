//! Here we use shape primitives to build meshes in a 2D rendering context, making each mesh a certain color by giving that mesh's entity a material based off a `Color`.
//!
//! A shape primitive is a, it is not a mesh on its own. A circle can be defined with a radius, i.e. `Circle::new(50.0)`, but rendering tends to happen with triangles. So we need to turn shape descriptions into meshes.
//!
//! Thankfully, we can add shape primitives directly to `Assets<Mesh>` because `Mesh` implements `From` for shape primitives and `Assets<T>::add` can be given any value that can be "turned into" `T`.
//!
//! Meshes are better known for their use in 3D rendering, but we can use then in a 2D context too. Without a third dimension, the models we're building are flat shapes. These are still very useful for collision, "vector-style" graphics, or as a basis for where to apply a shader.
//!
//! We apply a material to the shape by first making a `Color` then calling `Assets<ColorMaterial>::add` with that color as its argument, which will create a material from that color through the same process `Assets<Mesh>::add` can take a shape primitive.
//!
//! Both the mesh and material need to be wrapped in respective "newtypes". The mesh and material are currently `Handle<Mesh>` and `Handle<ColorMaterial>` at the moment, which are not components. Handles are put behind "newtypes" to make it easier to them as required components or to give them new required components and side-step the orphan rule. All we need to do to make them components is wrap the mesh handle and the material handle in `Mesh2d` and `MeshMaterial2d` respectively.
//!
//! You can toggle wireframes with the space bar except on wasm. Wasm does not support
//! `POLYGON_MODE_LINE` on the gpu.

use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::sprite::{Wireframe2dConfig, Wireframe2dPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        #[cfg(not(target_arch = "wasm32"))]
        Wireframe2dPlugin::default(),
    ))
    .add_systems(Startup, setup);
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, toggle_wireframe);
    app.run();
}

const X_EXTENT: f32 = 900.;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let shapes = [
        meshes.add(Circle::new(50.0)),
        meshes.add(CircularSector::new(50.0, 1.0)),
        meshes.add(CircularSegment::new(50.0, 1.25)),
        meshes.add(Ellipse::new(25.0, 50.0)),
        meshes.add(Annulus::new(25.0, 50.0)),
        meshes.add(Capsule2d::new(25.0, 50.0)),
        meshes.add(Rhombus::new(75.0, 100.0)),
        meshes.add(Rectangle::new(50.0, 100.0)),
        meshes.add(RegularPolygon::new(50.0, 6)),
        meshes.add(Triangle2d::new(
            Vec2::Y * 50.0,
            Vec2::new(-50.0, -50.0),
            Vec2::new(50.0, -50.0),
        )),
    ];
    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        // Distribute colors evenly across the rainbow.
        let color = Color::hsl(360. * i as f32 / num_shapes as f32, 0.95, 0.7);

        commands.spawn((
            Mesh2d(shape),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(
                // Distribute shapes from -X_EXTENT/2 to +X_EXTENT/2.
                -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                0.0,
                0.0,
            ),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        Text::new("Press space to toggle wireframes"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<Wireframe2dConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}
