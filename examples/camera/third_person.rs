//! This example shows how to implement a third person camera.
use bevy::input::mouse::{MouseButton, MouseMotion, MouseWheel};
use bevy::prelude::*;
use std::f32::consts::PI;

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(OldCursorPosition(Vec2::ZERO))
        .add_plugins(DefaultPlugins)
        .add_plugin(ThirdPersonCameraPlugin)
        .add_startup_system(startup)
        .add_system(mouse_control)
        .add_system(
            tp_camera_clamp
                // In order to avoid "overshooting" when you drag the camera beyond
                // maximum or minimum values, clamping must happen after update_tp
                // and before update_transform.
                //
                // You can get the "overshooting" effect by removing after update_tp
                // constraing and adding an after update_transform constraint, or
                // you can get it randomly if you don't add any constraints.
                .after(ThirdPersonCameraPlugin::update_tp)
                .before(ThirdPersonCameraPlugin::update_transform),
        )
        .run();
}

/// A third person camera with a frame of reference defined by "north" and
/// "zeroth meridian" vectors and spherical coordinates defined by the "state"
/// vector.
#[derive(Component, Debug)]
struct TPCamera {
    // vector pointing from the center towards the "north pole"
    north: Vec3,
    // vector pointing from the center towards the intersection of "zeroth
    // meridian" and the "equator"
    zeroth_meridian: Vec3,
    // camera state vector in spherical coordinates:
    //
    // x: longitude y: latitude z: distance from center
    pub state: Vec3,
}

impl Default for TPCamera {
    fn default() -> TPCamera {
        TPCamera {
            north: Vec3::Z,
            zeroth_meridian: Vec3::Y,
            state: Vec3::new(0.0, 0.0, 1.0),
        }
    }
}

#[derive(Bundle, Default)]
struct TPCameraBundle {
    tp: TPCamera,
    #[bundle]
    camera: Camera3dBundle,
}

/// An event type for controlling the third person camera.
#[derive(Debug)]
enum TPControlEvent {
    // Move the camera by a delta vector in spherical coordinates.
    Add(Vec3),
    // Teleport the camera to the new state vector.
    Set(Vec3),
}

struct OldCursorPosition(Vec2);

struct ThirdPersonCameraPlugin;

impl ThirdPersonCameraPlugin {
    /// This system reads `TPControlEvent`s and moves or teleports the camera
    /// accordingly.
    fn update_tp(mut control: EventReader<TPControlEvent>, mut camera: Query<&mut TPCamera>) {
        let mut camera = camera.single_mut();
        let mut delta = Vec3::ZERO;
        let mut set = None;
        for e in control.iter() {
            match e {
                // We accumulate all deltas from Add events.
                TPControlEvent::Add(d) => delta += *d,
                // We assign variable set to the latest Set event. If there are
                // no Set events it remains None.
                TPControlEvent::Set(new_tp) => set = Some(new_tp),
            };
        }
        // If there are any Set events we ignore all Add events and execute the
        // latest state event.
        if let Some(new_tp) = set {
            camera.state = *new_tp;
        // If there are no Set events we add the cumulative delta from all of
        // the Add events that happened between previous frame and this one to
        // the camera state vector.
        } else {
            camera.state += delta;
        }
    }

    /// This system updates the actual `Transform` of the camera according to the
    /// spherical coordinates in `TPCamera`.
    fn update_transform(mut camera: Query<(&TPCamera, &mut Transform)>) {
        let (tp, mut transform) = camera.single_mut();
        // First we compute the "horizontal" longitude rotation component.
        //
        // A positive longitude angle will slide us along the equator "east" and
        // a negative angle "west".
        //
        // You might want to remember the "corkscrew rule". We are rotating
        // around the "north" axis vector, so you can imagine a corkscrew with
        // it's head in the center of "earth" pointing towards the north pole.
        //
        // If we rotate this corkscrew clockwise or "eastward" it will go
        // forward (towards the north pole) in the positive direction (hence the
        // positive angle). If we rotate it counter-clockwise or "westward" it
        // will go backwards (away from the north pole) or in the negative
        // direction (so the angle will be negative).
        let longitude_rot = Quat::from_axis_angle(tp.north, tp.state.x);
        // We slide the "zeroth meridian" vector along the equator until we
        // get a "meridian" unit vector pointing towards a point on the "earth's
        // surface" with longitude = tp.state.x and latitude = 0.0.
        let meridian = longitude_rot * tp.zeroth_meridian;
        // Now we need to compute the "vertical" latitude rotation component.
        //
        // In order to do that we need to rotate the meridian vector either
        // "northward" or "southward". So the rotation will happen in the plane
        // spanned by the "north" vector and the "meridian" vector.
        //
        // The axis vector for this rotation must be perpendicular to the plane
        // in which we need to rotate. And we can get it via cross product of
        // "meridian" vector and "north" vector.
        //
        // We want positive angles to correspond to moving "north" and negative
        // to moving "south", so by
        // https://en.wikipedia.org/wiki/Right-hand_rule the meridian vector
        // must come first in the cross product.
        let latitude_rot_axis = meridian.cross(tp.north);
        // Equipped with the axis vector, we can now compute the latitude
        // rotation component.
        let latitude_rot = Quat::from_axis_angle(latitude_rot_axis, tp.state.y);
        // Now we compute a unit vector starting at the center of "earth" and
        // ending at a point with spherical coordinates: lon = tp.state.x, lat =
        // tp.state.y.
        //
        // We call it "r" because it is almost a radius vector.
        let r = latitude_rot * meridian;
        // Using the cross product and the right-hand rule we compute the "up"
        // vector for the camera.
        let up = latitude_rot_axis.cross(r);
        // And finally we set the camera's position to the proper radius vector
        // = tp.state.z * r, and point it at the origin.
        *transform = Transform::from_translation(tp.state.z * r).looking_at(Vec3::ZERO, up);
    }
}

impl Plugin for ThirdPersonCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TPControlEvent>()
            .add_system(
                ThirdPersonCameraPlugin::update_tp
                    // update_tp happening before update_transform is important
                    // for clamping.
                    .before(ThirdPersonCameraPlugin::update_transform),
            )
            .add_system(ThirdPersonCameraPlugin::update_transform);
    }
}

/// This system converts bevy mouse input events into `TPControlEvent`s that are
/// then interpreted by the `update_tp` system.
fn mouse_control(
    time: Res<Time>,
    mut old_cursor_position: ResMut<OldCursorPosition>,
    mut windows: ResMut<Windows>,
    buttons: Res<Input<MouseButton>>,
    mut scroll: EventReader<MouseWheel>,
    mut motion: EventReader<MouseMotion>,
    mut control: EventWriter<TPControlEvent>,
) {
    // In the end we multiply the delta by the time between frames in seconds,
    // so sensitivities are sort of like velocities and have 1/second in their
    // dimensions.
    const LINE_ZOOM_SENSITIVITY: f32 = 20.0; // units per (line * second)
    const PIXEL_ZOOM_SENSITIVITY: f32 = LINE_ZOOM_SENSITIVITY / 16.0; // units per (pixel * second)
    const SENSITIVITY_DEG: f32 = 10.0; // degrees per (pixel * second)
    const SENSITIVITY: f32 = SENSITIVITY_DEG / 180.0 * PI; // radians per (pixel * second)

    let mut delta = Vec3::ZERO;

    // We accumulate all scroll wheel events that happened between the previous
    // frame and this one and we add them up to the z "distance" component of
    // delta.
    for e in scroll.iter() {
        use bevy::input::mouse::MouseScrollUnit;
        let sensitivity = match e.unit {
            MouseScrollUnit::Line => LINE_ZOOM_SENSITIVITY,
            MouseScrollUnit::Pixel => PIXEL_ZOOM_SENSITIVITY,
        };
        delta.z -= e.y * sensitivity;
    }
    let window = windows.get_primary_mut().unwrap();
    if buttons.just_pressed(MouseButton::Left) {
        if let Some(cursor_position) = window.cursor_position() {
            // We save the cursor starting position, so we can return to it
            // after the button is released.
            old_cursor_position.0 = cursor_position;
        }
        window.set_cursor_visibility(false);
        // Even though the cursor is teleported back to it's starting position
        // when the button is released, we still lock it so the cursor can't
        // leave the window and interact with other windows while we are
        // rotating the camera.
        window.set_cursor_lock_mode(true);
    }
    if buttons.just_released(MouseButton::Left) {
        window.set_cursor_visibility(true);
        window.set_cursor_lock_mode(false);
        window.set_cursor_position(old_cursor_position.0);
    }
    if buttons.pressed(MouseButton::Left) {
        // We accumulate all mouse motions multiplied by sensitivity that
        // happened between the previous frame and this one in the (x, y)
        // "longitude" and "latitude" components delta.
        for e in motion.iter() {
            delta += (e.delta * SENSITIVITY).extend(0.0);
        }
    }
    // In order to get the "dragging a third person camera around a character"
    // effect we want the camera to rotate "west" when we drag it to the right
    // and "east" when we drag it to the left. So we invert the x (longitude)
    // component of our delta.
    delta.x = -delta.x;
    // We want to have a consistent sensitivity regardless of framerate, so we
    // account for time between frames.
    delta *= time.delta_seconds();
    if delta != Vec3::ZERO {
        // Now we send the third person camera control event that will be read
        // by the update_tp system, which will in turn move the camera delta.x
        // radians in longitude, delta.y radians in latitude, and delta.z units
        // in distance.
        control.send(TPControlEvent::Add(delta));
    }
    // Teleport back to starting position if right mouse button is pressed.
    if buttons.just_pressed(MouseButton::Right) {
        control.send(TPControlEvent::Set(Vec3::new(0.0, 0.0, 2.0)));
    }
}

/// This system limits camera movement in "latitude" and in "distance".
fn tp_camera_clamp(mut camera: Query<&mut TPCamera>) {
    const MIN_LATITUDE_DEG: f32 = 0.0;
    const MAX_LATITUDE_DEG: f32 = 90.0;

    const MIN_LATITUDE: f32 = MIN_LATITUDE_DEG / 180.0 * PI;
    const MAX_LATITUDE: f32 = MAX_LATITUDE_DEG / 180.0 * PI;

    const MIN_DISTANCE: f32 = 2.0;
    const MAX_DISTANCE: f32 = 10.0;

    let mut camera = camera.single_mut();

    camera.state.y = camera.state.y.clamp(MIN_LATITUDE, MAX_LATITUDE);
    camera.state.z = camera.state.z.clamp(MIN_DISTANCE, MAX_DISTANCE);
}

/// Startup system that creates a plane for ground, cube for a "character", and
/// a third person camera attached to the cube as a child.
fn startup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    // We add a plane representing the ground.
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
        material: materials.add(Color::rgb(0.7, 0.7, 0.7).into()),
        transform: Transform::from_rotation(Quat::from_rotation_x(PI / 2.0))
            .with_translation(Vec3::new(0.0, 0.0, -0.501)),
        ..default()
    });
    // We add a cube representing a character.
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.8, 0.8).into()),
            ..default()
        })
        .with_children(|parent| {
            // A camera is attached to the "character" as a child, so if the
            // "character" moves the camera will move with it.
            parent.spawn_bundle(TPCameraBundle::default());
        });
}
