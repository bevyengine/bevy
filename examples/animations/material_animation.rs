use bevy::animation::{AddAnimated, Animator, Clip};
use bevy::{math::curves::CurveVariableLinear, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .register_animated_asset::<StandardMaterial>()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clips: ResMut<Assets<Clip>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create clip
    let mut clip = Clip::default();
    clip.add_track_at_path(
        "@Handle<StandardMaterial>.albedo",
        CurveVariableLinear::from_line(0.0, 1.0, Color::WHITE, Color::ORANGE_RED),
    );
    let clip_handle = clips.add(clip);

    // Create the animator and add the clip
    let mut animator = Animator::default();
    animator.add_layer(clip_handle, 1.0);

    // Animated sphere
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 1.0,
                subdivisions: 5,
            })),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            material: materials.add(Color::rgb(0.1, 0.05, 0.0).into()),
            ..Default::default()
        })
        .insert(animator);

    // Camera and Light
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_matrix(Mat4::face_toward(
            Vec3::new(-3.0, 5.0, 8.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )),
        ..Default::default()
    });
}
