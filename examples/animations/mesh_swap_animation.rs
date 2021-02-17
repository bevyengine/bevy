use bevy::animation::{tracks::TrackVariableLinear, AddAnimated, Animator, Clip};
use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .register_animated_asset::<Mesh>()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clips: ResMut<Assets<Clip>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let sphere = meshes.add(Mesh::from(shape::Icosphere {
        radius: 1.0,
        subdivisions: 5,
    }));

    // TODO: more shapes will be nice!

    // Create clip
    let mut clip = Clip::default();
    clip.add_track_at_path(
        "@Handle<Mesh>",
        TrackVariableLinear::new(
            vec![0.0, 0.5, 1.0],
            vec![cube.clone(), sphere, cube.clone()],
        ),
    );
    let clip_handle = clips.add(clip);

    // Create the animator and add the clip
    let mut animator = Animator::default();
    animator.add_layer(clip_handle, 1.0);

    commands
        .spawn(PbrBundle {
            mesh: cube,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            material: materials.add(Color::ANTIQUE_WHITE.into()),
            ..Default::default()
        })
        .with(animator);

    commands
        .spawn(LightBundle {
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        })
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
