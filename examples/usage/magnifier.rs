//! Demonstrates zooming part of the screen with SubCameraView
use bevy::{
    camera::Viewport, input::mouse::AccumulatedMouseScroll, prelude::*, window::PrimaryWindow,
};

#[derive(Component)]
struct Magnification(f32);

#[derive(Component)]
struct ViewportSize(UVec2);

#[derive(Component)]
struct DebugText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_magnification, update_sub_view, update_debug_text),
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

    // Main Camera
    commands.spawn((Camera3d::default(), IsDefaultUiCamera, transform));

    // Magnifier Camera
    let physical_size = UVec2::new(200, 200);
    commands.spawn((
        Camera3d::default(),
        Camera {
            viewport: Some(Viewport {
                physical_size,
                ..default()
            }),
            order: 1,
            ..default()
        },
        ViewportSize(physical_size),
        Magnification(0.25),
        transform,
    ));

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

fn update_magnification(
    mut magnification: Single<&mut Magnification>,
    scroll: Res<AccumulatedMouseScroll>,
) {
    // Each scroll delta changes the scale by 5%
    const ZOOM_PERCENT: f32 = 0.05;
    magnification.0 *= ops::powf(ZOOM_PERCENT + 1.0, -scroll.delta.y);

    // Going above 1.0 makes the magnified image smaller rather than larger, which looks weird
    magnification.0 = magnification.0.min(1.0);
}

fn update_sub_view(
    background_camera: Single<&Camera, With<IsDefaultUiCamera>>,
    camera: Single<
        (&mut Camera, &mut Projection, &ViewportSize, &Magnification),
        Without<IsDefaultUiCamera>,
    >,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let (mut camera, mut projection, viewport_size, magnification) = camera.into_inner();

    let Some(cursor) = window.physical_cursor_position() else {
        return;
    };

    // Ensure that the full view of the magnifier camera has the same aspect ratio as the background camera.
    // This is necessary to ensure that the part of the image covered by the viewport is the same as
    // the part of the image covered by the sub view.
    // The viewport covers the part of the image displayed by the background camera at that position in the window,
    // while the sub view covers the part of the image at the given offset within the camera's full projection.
    // If the aspect ratios differed, the parts of the image at the same relative (horizontal) offset would be different.
    if let Some(size) = background_camera.logical_viewport_size() {
        projection.update(size.x, size.y);
    }

    let viewport = camera.viewport.as_mut().unwrap();
    // Bevy can automatically resize the viewport if the window is shrunk such that the viewport goes offscreen,
    // so we set the viewport size every frame to recover from this possibly happening
    viewport.physical_size = viewport_size.0;
    let viewport_size = viewport_size.0.as_vec2();
    let viewport_half_size = viewport_size / 2.0;

    let window_size = window.physical_size().as_vec2();

    // Clamp cursor position to a margin within the window bounds, so the viewport doesn't go off screen
    let cursor = cursor.clamp(viewport_half_size, window_size - viewport_half_size);

    // Center viewport on cursor
    let viewport_pos = cursor - viewport_half_size;
    viewport.physical_position = viewport_pos.as_uvec2();

    // Normalise the position to the 0..1 range the sub view expects
    let mut offset = cursor / window_size;

    // A subview scale of 1.0 makes the full window image visible within the viewport (i.e. very "zoomed out").
    // For 1x magnification, so the viewport shows the same image as just the background area it covers,
    // the subview scale would be `viewport_size.y / window_size.y`.
    // So, we scale that value by the intended magnification to get the actual subview scale.
    let size = magnification.0 * (viewport_size / window_size);

    // `viewport_size / window_size` is the fraction of the window that the viewport covers.
    // At 1x magnification we want the sub view to render this fraction of the full view, so that the image is the same.
    // The fraction will be different along the x and y axes though, so we must pick one.
    // However, the sub view scale is the fraction of that camera's full projection, and the "full view" we are thinking of here
    // is actually the projection of the other camera (that renders the background image).
    // This means we must ensure that at least one of the axes of the projections of both of these cameras is the same,
    // so that we get the correct result from specifying the fraction of the "full view" we want as the fraction that the sub view uses.
    // Fortunately, this is already the case for us, as both projections were initialised with all the same default parameters,
    // except the aspect ratio. The aspect ratio only affects the x axis, so we pick the y axis instead.
    let scale = size.y;

    // Offset by half the subview size so that scaling the subview scales around the center rather than the top-left.
    let half_size = size / 2.0;
    offset -= half_size;

    // Retrieve and mutate the sub view instead of directly updating its value, so that Bevy's `camera_system`
    // can manage the aspect ratio parameter for us.
    let sub_view = camera.sub_camera_view.get_or_insert_default();
    sub_view.scale = scale;
    sub_view.offset = offset;
    // Set the sub view's aspect ratio to Some, so we can manually control the full projection's aspect ratio
    sub_view.aspect_ratio.get_or_insert_default();
}

fn update_debug_text(
    mut text: Single<&mut TextSpan, With<DebugText>>,
    camera: Single<(&Camera, &Magnification)>,
) {
    text.0 = format!(
        "magnification: {:?}\nsub_camera_view: {:?}",
        camera.1 .0, camera.0.sub_camera_view
    );
}
