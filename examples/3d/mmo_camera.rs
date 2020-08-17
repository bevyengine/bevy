use bevy:: {
    prelude::*,
    input::mouse::MouseMotion,
};

struct PlayerControl;

struct FaceTowardsParent {
    offset: Vec3,
    distance: f32,
    angle: f32,
}

impl Default for FaceTowardsParent {
    fn default() -> Self {
        FaceTowardsParent {
            offset: Vec3::default(),
            distance: 10.0,
            angle: 30.0f32.to_radians(),
        }
    }
}

#[derive(Default)]
struct InputManager {
    movement: Vec2,
    look: Vec2,
    camera_zoom: f32,
}

#[derive(Default)]
struct State {
    mouse_motion_event_reader: EventReader<MouseMotion>,
}

fn main() {
    App::build()
    .add_resource(Msaa { samples: 4 })
    .init_resource::<State>()
    .init_resource::<InputManager>()
    .add_default_plugins()
    .add_startup_system(setup.system())
    .add_system(clear_input_manager.system())
    .add_system(process_mouse_events.system())
    .add_system(process_keys.system())
    .add_system(rotate_player.system())
    .add_system(move_player.system())
    .add_system(player_camera_target.system())
    .add_system(perform_camera_zoom.system())
    .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let cube_mat_handle = materials.add({
        let mut cube_material: StandardMaterial = Color::rgb(1.0, 1.0, 1.0).into();
        cube_material.shaded = true;
        cube_material
    });

    commands
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: cube_mat_handle.clone(),
            translation: Translation::new(0.0, 1.0, 0.0),
            ..Default::default()
        })
        .with(PlayerControl)
        .with(Rotation)
        .with_children(|parent| {
            parent
                .spawn(Camera3dComponents::default())
                .with(Rotation)
                .with(FaceTowardsParent::default());
        })
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
            material: materials.add(Color::rgb(0.7, 0.3, 0.0).into()),
            ..Default::default()
        })
        .spawn(LightComponents {
            translation: Translation::new(4.0, 5.0, 4.0),
            ..Default::default()
        });
}

fn clear_input_manager(mut input: ResMut<InputManager>) {
    input.look = Vec2::zero(); // This one is event driven so needs to be cleared at the start of the frame.
}

fn process_mouse_events(
    mut state: ResMut<State>, 
    mut input: ResMut<InputManager>, 
    mouse_motion_events: Res<Events<MouseMotion>>
) {
    for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
        input.look = event.delta;
    }
}

fn process_keys(input: Res<Input<KeyCode>>, mut manager: ResMut<InputManager>) {
    manager.movement = Vec2::zero();
    if input.pressed(KeyCode::W) { *manager.movement.x_mut() += 1.; }
    if input.pressed(KeyCode::S) { *manager.movement.x_mut() -= 1.; }
    if input.pressed(KeyCode::D) { *manager.movement.y_mut() += 1.; }
    if input.pressed(KeyCode::A) { *manager.movement.y_mut() -= 1.; }

    if manager.movement != Vec2::zero() { manager.movement.normalize(); }


    manager.camera_zoom = 0.;
    if input.pressed(KeyCode::R) { manager.camera_zoom += 1.; }
    if input.pressed(KeyCode::F) { manager.camera_zoom -= 1.; }
}

fn rotate_player(
    time: Res<Time>,
    input: Res<InputManager>,
    mut query: Query<(&PlayerControl, &mut Rotation)>,
    mut cams: Query<&mut FaceTowardsParent>,
) {
    let rot = input.look * time.delta_seconds;

    for (_, mut rotation) in &mut query.iter() {
        rotation.0 *= Quat::from_rotation_y(-rot.x());
    }

    for mut cam in &mut cams.iter() {
        cam.angle = (cam.angle - rot.y()).max(1f32.to_radians()).min(std::f32::consts::PI - 1f32.to_radians());
    }
}

fn perform_camera_zoom(
    time: Res<Time>, 
    input: Res<InputManager>, 
    mut cams: Query<&mut FaceTowardsParent>
) {
    let dist = input.camera_zoom * time.delta_seconds;

    for mut cam in &mut cams.iter() {
        cam.distance += dist * 5.0;
        cam.distance = cam.distance.max(1.0).min(30.0);
    }
}

fn move_player(
    time: Res<Time>, 
    input: Res<InputManager>, 
    mut query: Query<(&PlayerControl, &Transform, &mut Translation)>
) {
    let movement = input.movement * time.delta_seconds;

    for (_, transform, mut translation) in &mut query.iter() {
        let fwd = transform.value.z_axis().truncate();
        let right = -transform.value.x_axis().truncate();

        let delta: Vec3 = (fwd * movement.x() + right * movement.y()).into();
        if delta == Vec3::zero() { continue; }
        let delta = delta.normalize() * time.delta_seconds * 10.0;

        translation.0 += delta;
    }
}

fn player_camera_target(mut look: Query<(&FaceTowardsParent, &mut Translation, &mut Rotation)>) {
    for (face_towards, mut translation, mut rotation) in &mut look.iter() {
        let to = Vec3::new(0., face_towards.angle.cos(), -face_towards.angle.sin()).normalize();
        translation.0 = to * face_towards.distance + face_towards.offset;

        let look = Mat4::face_toward(translation.0, Vec3::zero(), Vec3::new(0.0, 1.0, 0.0));
        rotation.0 = look.to_scale_rotation_translation().1;
    }
}