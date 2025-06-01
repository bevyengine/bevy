//! Here we cover the basics of how to set up a basic 3D scene in Bevy. This is how a lot of 3D Bevy apps start out in the real world before being iterated on: a camera, a light, and a couple of things to look at (for us here, a cube and a flat circle.)
//!
//! To create any of those objects, we need to use [`Commands`] to "spawn" entities. This is the canonical way of creating new entities in Bevy. Keep in mind spawning doesn't happen immediately! Commands are a "message" that Bevy holds on to until the end of each [`Schedule`] "phase."
//!
//! [`Commands`] are accessed through a "System", which in Bevy is just a function that takes specific kinds of arguments and gets manually "registered" so that Bevy knows we want to run it. We'll also want this system to run once at the beginning of the program in the [`Startup`] schedule label.
//!
//! Cameras are necessary for rendering, and we want to be rendering in 3D so we will need to spawn a [`Camera3d`]. There's ways you can configure the camera for your specific needs, but for now we'll go for the default with [`Camera3d::default()`][Camera3d::default].
//!
//! We also want to place the camera away from the center of the 3D space looking at that center, so that we can spawn 3D objects close to the "origin" `(0, 0, 0)` and know the camera will be looking at them. Creating a transform that's at a translation away from the center and then calling [`Transform::looking_at`] on that transform achieves this, and we add that transform alongside our `Camera3d` in a tuple using the `commands.spawn` method to attach both components to a new entity.
//!
//! For spawning in 3D objects, at least ones generated from primitive shapes, we need access to how bevy stores meshes (the most common data for 3D objects) and materials (what colors, textures those objects have). We do this in bevy by getting Mutable access to Resources ([`ResMut`]) that store `Assets` that manage `Mesh` and `StandardMaterial` data, which are both Mesh Data and Material Data respectively.
//!
//! [`Assets<T>`] is a way of processing and storing "asset"-like data. You "give" it data with the [`.add`][Assets::add] method and it gives you back a [`Handle<T>`], this prevents storing duplicate copies of the same data and removes any assets from memory that we're no longer using.
//!
//! The only shapes we want in this basic scene are a [`Cuboid`] and a [`Circle`]. We can load both of these as assets by using them as arguments to `meshes.add` because the [`Assets<T>`] resource handles anything that can be "converted" to `T`. There's an implementation for processing primitive shapes into meshes, so it does that for us. But [`Handle<Mesh>`] is not a component on its own (as some entities might want multiple meshes or other assets for different purposes) so in this circumstance to use it as a rendering mesh we need to wrap it in a [`Mesh3d`].
//!
//! Applying a [`Color`] to a mesh is a similar story: we can create a new material by calling [`materials.add`][Assets::add] on a [`Color`], which gets converted to a [`StandardMaterial`] because there's an implementation for converting [`Color`] to [`StandardMaterial`]. Again, [`Handle<StandardMaterial>`] is not a component (for the same reason as [`Mesh3d`]) so we need to wrap it in [`MeshMaterial3d`].
//!
//! With a camera and meshes spawned, we now need a light. [`PointLight`] is a Light that behaves like it's coming from a Point: think lightbulbs or lit matches. We need to make sure it's not "within" any of the meshes we've spawned because then the light will be blocked by those meshes, so we spawn it at a similar distance from the center of the scene as the camera, but in a different spot. We enable shadow shadows in the [`PointLight`]'s constructor and fill the rest with default values, one of the possible values you could choose is a different strength for the light.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
