use bevy::{animation::Animator, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(anim_set.system().label("anim_set"))
        .add_system(anim_blending.system().after("anim_set"))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 20.0 })),
        transform: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        material: materials.add(Color::rgb(0.1, 0.05, 0.0).into()),
        ..Default::default()
    });

    // light
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_matrix(Mat4::face_toward(
            Vec3::new(-3.0, 5.0, 8.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )),
        ..Default::default()
    });

    // character
    commands.spawn_scene(asset_server.load("models/character_medium/character_medium.gltf#Scene0"));
}

fn anim_set(asset_server: Res<AssetServer>, mut animators_query: Query<(&mut Animator,)>) {
    // Load animations and set them to the animator
    for (mut animator,) in animators_query.iter_mut() {
        if animator.clips().is_empty() {
            animator.add_layer(
                asset_server.load("models/character_medium/idle.gltf#Anim0"),
                1.0,
            );
            animator.add_layer(
                asset_server.load("models/character_medium/run.gltf#Anim0"),
                1.0,
            );
        }
    }
}

#[derive(Default, Debug)]
struct PingPong {
    pong: bool,
}

fn anim_blending(
    mut ping_pong: Local<PingPong>,
    time: Res<Time>,
    mut animators_query: Query<(&mut Animator,)>,
) {
    // Perform a simple ping pong blending between the run and idle animation
    for (mut animator,) in animators_query.iter_mut() {
        let dw = if ping_pong.pong {
            time.delta_seconds() / 5.0
        } else {
            -time.delta_seconds() / 5.0
        };

        let mut w = animator.layers[0].weight;
        w = (w + dw).min(1.0).max(0.0);

        if ping_pong.pong {
            if w >= (1.0 - 1e-2) {
                ping_pong.pong = false;
            }
        } else if w <= 1e-2 {
            ping_pong.pong = true;
        }

        animator.layers[0].weight = w; // Idle animation layer
        animator.layers[1].weight = 1.0 - w; // Run animation layer
    }
}
