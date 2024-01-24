use bevy::pbr::irradiance_volume::IrradianceVolume;
use bevy::prelude::*;

// Rotation speed in radians per frame.
const ROTATION_SPEED: f32 = 0.005;

static DISABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Press Space to disable the irradiance volume";
static ENABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Press Space to enable the irradiance volume";

static STOP_ROTATION_HELP_TEXT: &str = "Press Enter to stop rotation";
static START_ROTATION_HELP_TEXT: &str = "Press Enter to start rotation";

// The mode the application is in.
#[derive(Resource)]
struct AppStatus {
    irradiance_volume_present: bool,
    // Whether the user has requested the scene to rotate.
    rotating: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<AppStatus>()
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, update_text.after(rotate_camera))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneBundle {
        scene: asset_server
            .load("models/IrradianceVolumeExample/IrradianceVolumeExample.glb#Scene0"),
        ..SceneBundle::default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10.012, 4.8605, 13.281).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands
        .spawn(SpatialBundle {
            transform: Transform::from_matrix(Mat4::from_cols_array_2d(&[
                [2.5328817, 0.0, 0.0, 0.0],
                [0.0, 0.0, -2.5328817, 0.0],
                [0.0, 2.0, 0.0, 0.0],
                [0.15830529, 1.1666666, -0.15830529, 1.0],
            ])),
            ..SpatialBundle::default()
        })
        .insert(IrradianceVolume {
            voxels: asset_server.load::<Image>("irradiance_volumes/Example.vxgi.ktx2"),
            intensity: 1.0,
        })
        .insert(LightProbe);

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 250000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0762, 5.9039, 1.0055),
        ..default()
    });
}

// A system that updates the help text.
fn update_text(
    mut text_query: Query<&mut Text>,
    app_status: Res<AppStatus>,
    asset_server: Res<AssetServer>,
) {
    for mut text in text_query.iter_mut() {
        *text = app_status.create_text(&asset_server);
    }
}

impl AppStatus {
    // Constructs the help text at the bottom of the screen based on the
    // application status.
    fn create_text(&self, asset_server: &AssetServer) -> Text {
        let irradiance_volume_help_text = if self.irradiance_volume_present {
            DISABLE_IRRADIANCE_VOLUME_HELP_TEXT
        } else {
            ENABLE_IRRADIANCE_VOLUME_HELP_TEXT
        };

        let rotation_help_text = if self.rotating {
            STOP_ROTATION_HELP_TEXT
        } else {
            START_ROTATION_HELP_TEXT
        };

        Text::from_section(
            format!("{}\n{}", irradiance_volume_help_text, rotation_help_text),
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 24.0,
                color: Color::ANTIQUE_WHITE,
            },
        )
    }
}

// Rotates the camera a bit every frame.
fn rotate_camera(mut camera_query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in camera_query.iter_mut() {
        transform.translation = Vec2::from_angle(ROTATION_SPEED)
            .rotate(transform.translation.xz())
            .extend(transform.translation.y)
            .xzy();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            irradiance_volume_present: true,
            rotating: true,
        }
    }
}
