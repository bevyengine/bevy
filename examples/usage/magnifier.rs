//! Demonstrates zooming part of the screen with [`SubCameraView`].
//!
//! Also see the `camera_sub_view` example for more information about sub views.
use bevy::{
    camera::{SubCameraView, SubViewSourceProjection, Viewport},
    input::mouse::AccumulatedMouseScroll,
    prelude::*,
    window::PrimaryWindow,
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
    asset_server: Res<AssetServer>,
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

    // Tiny Bevy logo
    let scale = 0.05;
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(2.0 * scale, 0.5 * scale))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("branding/bevy_logo_dark.png")),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        })),
        // Slightly further along the z axis to avoid z fighting with the cube
        Transform::from_xyz(0.0, 0.5, 0.501),
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
    let main_camera = commands
        .spawn((Camera3d::default(), IsDefaultUiCamera, transform))
        .id();

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
        // The `SubViewSourceProjection` component lets us use the other camera's projection instead, but while retaining our
        // own viewport's aspect ratio, which is needed to massively simplify the math we have to do. This is not just for
        // magnification, but any time a sub view is used to overlay a visual alteration on top of an image with a different
        // aspect ratio.
        SubViewSourceProjection(main_camera),
        ViewportSize(physical_size),
        Magnification(0.25),
        transform,
    ));

    // Debug text
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        DebugText,
    ));
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
    camera: Single<(&mut Camera, &ViewportSize, &Magnification), Without<IsDefaultUiCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let (mut camera, viewport_size, magnification) = camera.into_inner();

    let Some(cursor) = window.physical_cursor_position() else {
        return;
    };

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

    // A subview scale of 1.0 makes the full window image visible within the much smaller viewport (i.e. very "zoomed out").
    // For 1x magnification, so the viewport shows the same image as just the background area it covers,
    // the subview scale would be `viewport_size.y / window_size.y`.
    // So, we scale that value by the intended magnification to get the actual subview scale that we use.
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

    camera.sub_camera_view = Some(SubCameraView { scale, offset });
}

fn update_debug_text(
    mut text: Single<&mut Text, With<DebugText>>,
    camera: Single<(&Camera, &Magnification)>,
) {
    text.0 = format!(
        "magnification: {:?}\nsub_camera_view: {:?}",
        camera.1 .0, camera.0.sub_camera_view
    );
}
