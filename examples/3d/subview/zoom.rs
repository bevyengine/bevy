//! Demonstrates zooming the whole screen with SubCameraView
use bevy::{
    input::mouse::AccumulatedMouseScroll, prelude::*, render::camera::SubCameraView,
    window::PrimaryWindow,
};

#[derive(Resource)]
struct IsSubViewActive(bool);

#[derive(Component)]
struct Scale(f32);

#[derive(Component)]
struct DebugText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(IsSubViewActive(true))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (toggle_sub_view, update_sub_view, update_debug_text),
        )
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    let transform = Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y);

    // Camera
    commands.spawn((Camera3d::default(), Scale(0.25), transform));

    // Debug text
    commands
        .spawn((
            Text::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|children| {
            children.spawn(TextSpan::new("Press space to toggle sub view"));
            children.spawn(TextSpan::new("\n"));
            children.spawn((TextSpan::default(), DebugText));
        });
}

fn toggle_sub_view(mut is_active: ResMut<IsSubViewActive>, inputs: Res<ButtonInput<KeyCode>>) {
    if inputs.just_pressed(KeyCode::Space) {
        is_active.0 = !is_active.0;
    }
}

fn update_sub_view(
    camera: Single<(&mut Camera, &mut Projection, &mut Scale)>,
    window: Single<&Window, With<PrimaryWindow>>,
    is_active: Res<IsSubViewActive>,
    scroll: Res<AccumulatedMouseScroll>,
) {
    let (mut camera, mut projection, mut scale) = camera.into_inner();

    if is_active.0 {
        // Each scroll delta changes the scale by 5%
        const ZOOM_PERCENT: f32 = 0.05;
        scale.0 *= ops::powf(ZOOM_PERCENT + 1.0, -scroll.delta.y);
        let scale = scale.0;

        if let Some(cursor) = window.physical_cursor_position() {
            // Offset is the offset of the top-left corner of the sub-view from the top-left corner of the viewport.
            // Scale it so that the bottom-right corner of the sub-view can't go outside of the screen.
            let offset = (cursor / window.physical_size().as_vec2()) * (1.0 - scale);
            let sub_view = camera.sub_camera_view.get_or_insert_default();
            sub_view.scale = scale;
            sub_view.offset = offset;
        }

        if let Some(size) = camera.logical_viewport_size() {
            projection.update(size.x, size.y);
        }
    } else {
        camera.sub_camera_view = None;
    }
}

fn update_debug_text(mut text: Single<&mut TextSpan, With<DebugText>>, camera: Single<&Camera>) {
    text.0 = format!("sub_camera_view: {:?}", camera.sub_camera_view);
}
