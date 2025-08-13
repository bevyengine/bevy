//! Shows how to zoom orthographic and perspective projection cameras.

use std::{f32::consts::PI, ops::Range};

use bevy::{camera::ScalingMode, input::mouse::AccumulatedMouseScroll, prelude::*};

#[derive(Debug, Resource)]
struct CameraSettings {
    /// The height of the viewport in world units when the orthographic camera's scale is 1
    pub orthographic_viewport_height: f32,
    /// Clamp the orthographic camera's scale to this range
    pub orthographic_zoom_range: Range<f32>,
    /// Multiply mouse wheel inputs by this factor when using the orthographic camera
    pub orthographic_zoom_speed: f32,
    /// Clamp perspective camera's field of view to this range
    pub perspective_zoom_range: Range<f32>,
    /// Multiply mouse wheel inputs by this factor when using the perspective camera
    pub perspective_zoom_speed: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(CameraSettings {
            orthographic_viewport_height: 5.,
            // In orthographic projections, we specify camera scale relative to a default value of 1,
            // in which one unit in world space corresponds to one pixel.
            orthographic_zoom_range: 0.1..10.0,
            // This value was hand-tuned to ensure that zooming in and out feels smooth but not slow.
            orthographic_zoom_speed: 0.2,
            // Perspective projections use field of view, expressed in radians. We would
            // normally not set it to more than π, which represents a 180° FOV.
            perspective_zoom_range: (PI / 5.)..(PI - 0.2),
            // Changes in FOV are much more noticeable due to its limited range in radians
            perspective_zoom_speed: 0.05,
        })
        .add_systems(Startup, (setup, instructions))
        .add_systems(Update, (switch_projection, zoom))
        .run();
}

/// Set up a simple 3D scene
fn setup(
    asset_server: Res<AssetServer>,
    camera_settings: Res<CameraSettings>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            // We can set the scaling mode to FixedVertical to keep the viewport height constant as its aspect ratio changes.
            // The viewport height is the height of the camera's view in world units when the scale is 1.
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: camera_settings.orthographic_viewport_height,
            },
            // This is the default value for scale for orthographic projections.
            // To zoom in and out, change this value, rather than `ScalingMode` or the camera's position.
            scale: 1.,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Plane"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            // Turning off culling keeps the plane visible when viewed from beneath.
            cull_mode: None,
            ..default()
        })),
    ));

    commands.spawn((
        Name::new("Fox"),
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb")),
        ),
        // Note: the scale adjustment is purely an accident of our fox model, which renders
        // HUGE unless mitigated!
        Transform::from_translation(Vec3::splat(0.0)).with_scale(Vec3::splat(0.025)),
    ));

    commands.spawn((
        Name::new("Light"),
        PointLight::default(),
        Transform::from_xyz(3.0, 8.0, 5.0),
    ));
}

fn instructions(mut commands: Commands) {
    commands.spawn((
        Name::new("Instructions"),
        Text::new(
            "Scroll mouse wheel to zoom in/out\n\
            Space: switch between orthographic and perspective projections",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        },
    ));
}

fn switch_projection(
    mut camera: Single<&mut Projection, With<Camera>>,
    camera_settings: Res<CameraSettings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        // Switch projection type
        **camera = match **camera {
            Projection::Orthographic(_) => Projection::Perspective(PerspectiveProjection {
                fov: camera_settings.perspective_zoom_range.start,
                ..default()
            }),
            Projection::Perspective(_) => Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical {
                    viewport_height: camera_settings.orthographic_viewport_height,
                },
                ..OrthographicProjection::default_3d()
            }),
            _ => return,
        }
    }
}

fn zoom(
    camera: Single<&mut Projection, With<Camera>>,
    camera_settings: Res<CameraSettings>,
    mouse_wheel_input: Res<AccumulatedMouseScroll>,
) {
    // Usually, you won't need to handle both types of projection,
    // but doing so makes for a more complete example.
    match *camera.into_inner() {
        Projection::Orthographic(ref mut orthographic) => {
            // We want scrolling up to zoom in, decreasing the scale, so we negate the delta.
            let delta_zoom = -mouse_wheel_input.delta.y * camera_settings.orthographic_zoom_speed;
            // When changing scales, logarithmic changes are more intuitive.
            // To get this effect, we add 1 to the delta, so that a delta of 0
            // results in no multiplicative effect, positive values result in a multiplicative increase,
            // and negative values result in multiplicative decreases.
            let multiplicative_zoom = 1. + delta_zoom;

            orthographic.scale = (orthographic.scale * multiplicative_zoom).clamp(
                camera_settings.orthographic_zoom_range.start,
                camera_settings.orthographic_zoom_range.end,
            );
        }
        Projection::Perspective(ref mut perspective) => {
            // We want scrolling up to zoom in, decreasing the scale, so we negate the delta.
            let delta_zoom = -mouse_wheel_input.delta.y * camera_settings.perspective_zoom_speed;

            // Adjust the field of view, but keep it within our stated range.
            perspective.fov = (perspective.fov + delta_zoom).clamp(
                camera_settings.perspective_zoom_range.start,
                camera_settings.perspective_zoom_range.end,
            );
        }
        _ => (),
    }
}
