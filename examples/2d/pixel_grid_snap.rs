//! Shows how to create graphics that snap to the pixel grid by rendering to a texture in 2D

use bevy::{
    camera::visibility::RenderLayers,
    camera::RenderTarget,
    color::palettes::css::GRAY,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::WindowResized,
};

/// In-game resolution width.
const RES_WIDTH: u32 = 160;

/// In-game resolution height.
const RES_HEIGHT: u32 = 90;

/// Default render layers for pixel-perfect rendering.
/// You can skip adding this component, as this is the default.
const PIXEL_PERFECT_LAYERS: RenderLayers = RenderLayers::layer(0);

/// Render layers for high-resolution rendering.
const HIGH_RES_LAYERS: RenderLayers = RenderLayers::layer(1);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, (setup_camera, setup_sprite, setup_mesh))
        .add_systems(Update, (rotate, fit_canvas))
        .run();
}

/// Low-resolution texture that contains the pixel-perfect world.
/// Canvas itself is rendered to the high-resolution world.
#[derive(Component)]
struct Canvas;

/// Camera that renders the pixel-perfect world to the [`Canvas`].
#[derive(Component)]
struct InGameCamera;

/// Camera that renders the [`Canvas`] (and other graphics on [`HIGH_RES_LAYERS`]) to the screen.
#[derive(Component)]
struct OuterCamera;

#[derive(Component)]
struct Rotate;

fn setup_sprite(mut commands: Commands, asset_server: Res<AssetServer>) {
    // The sample sprite that will be rendered to the pixel-perfect canvas
    commands.spawn((
        Sprite::from_image(asset_server.load("pixel/bevy_pixel_dark.png")),
        Transform::from_xyz(-45., 20., 2.),
        Rotate,
        PIXEL_PERFECT_LAYERS,
    ));

    // The sample sprite that will be rendered to the high-res "outer world"
    commands.spawn((
        Sprite::from_image(asset_server.load("pixel/bevy_pixel_light.png")),
        Transform::from_xyz(-45., -20., 2.),
        Rotate,
        HIGH_RES_LAYERS,
    ));
}

/// Spawns a capsule mesh on the pixel-perfect layer.
fn setup_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Mesh2d(meshes.add(Capsule2d::default())),
        MeshMaterial2d(materials.add(Color::BLACK)),
        Transform::from_xyz(25., 0., 2.).with_scale(Vec3::splat(32.)),
        Rotate,
        PIXEL_PERFECT_LAYERS,
    ));
}

fn setup_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let canvas_size = Extent3d {
        width: RES_WIDTH,
        height: RES_HEIGHT,
        ..default()
    };

    // This Image serves as a canvas representing the low-resolution game screen
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // Fill image.data with zeroes
    canvas.resize(canvas_size);

    let image_handle = images.add(canvas);

    // This camera renders whatever is on `PIXEL_PERFECT_LAYERS` to the canvas
    commands.spawn((
        Camera2d,
        Camera {
            // Render before the "main pass" camera
            order: -1,
            target: RenderTarget::Image(image_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(GRAY.into()),
            ..default()
        },
        Msaa::Off,
        InGameCamera,
        PIXEL_PERFECT_LAYERS,
    ));

    // Spawn the canvas
    commands.spawn((Sprite::from_image(image_handle), Canvas, HIGH_RES_LAYERS));

    // The "outer" camera renders whatever is on `HIGH_RES_LAYERS` to the screen.
    // here, the canvas and one of the sample sprites will be rendered by this camera
    commands.spawn((Camera2d, Msaa::Off, OuterCamera, HIGH_RES_LAYERS));
}

/// Rotates entities to demonstrate grid snapping.
fn rotate(time: Res<Time>, mut transforms: Query<&mut Transform, With<Rotate>>) {
    for mut transform in &mut transforms {
        let dt = time.delta_secs();
        transform.rotate_z(dt);
    }
}

/// Scales camera projection to fit the window (integer multiples only).
fn fit_canvas(
    mut resize_events: EventReader<WindowResized>,
    mut projection: Single<&mut Projection, With<OuterCamera>>,
) {
    let Projection::Orthographic(projection) = &mut **projection else {
        return;
    };
    for event in resize_events.read() {
        let h_scale = event.width / RES_WIDTH as f32;
        let v_scale = event.height / RES_HEIGHT as f32;
        projection.scale = 1. / h_scale.min(v_scale).round();
    }
}
