//! Plugin to simulate and preview different types of
//! color blindness.

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetServer, Assets, Handle, HandleUntyped};
use bevy_core_pipeline::core_2d::Camera2dBundle;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Added,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::{Vec2, Vec3};
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::{Camera, RenderTarget},
    mesh::{shape, Mesh},
    prelude::Image,
    render_resource::{
        AsBindGroup, Extent3d, Shader, ShaderRef, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages,
    },
    view::RenderLayers,
};
use bevy_sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle};
use bevy_transform::prelude::Transform;
use bevy_ui::entity::UiCameraConfig;
use bevy_window::Windows;

/// Plugin to simulate and preview different types of
/// color blindness.
///
/// This lets you ensure that your game is accessible to all players by testing how it
/// will be seen under different conditions. While this is important,
/// please also consider not relying on color alone to convey important information to your players.
/// A common option is to add identifying symbols, like in the game
/// [Hue](https://gameaccessibilityguidelines.com/hue-colorblind-mode/).
///
/// Based on [Alan Zucconi's post](https://www.alanzucconi.com/2015/12/16/color-blindness/).
/// Supports: Normal, Protanopia, Protanomaly, Deuteranopia, Deuteranomaly,
/// Tritanopia, Tritanomaly, Achromatopsia, and Achromatomaly.
///
/// First, add the [`ColorBlindnessPlugin`] to your app, and add [`ColorBlindnessCamera`] to
/// your main camera.
///
/// You can change the selected mode by inserting [`ColorBlindnessParams`] before the plugin.
/// You can also skip this, and change the resource at any time in a system.
///
/// ```rust,no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins};
/// # use bevy_color_blindness::*;
/// # use bevy_core_pipeline::core_3d::Camera3dBundle;
/// # use bevy_ecs::system::Commands;
/// # use bevy_math::prelude::*;
/// # use bevy_transform::prelude::*;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .insert_resource(ColorBlindnessParams {
///             mode: Mode::Deuteranomaly,
///             enable: true,
///         })
///         // add the plugin
///         .add_plugin(ColorBlindnessPlugin)
///         .add_startup_system(setup)
///         .run();
/// }
///
/// fn setup(mut commands: Commands) {
///     // set up your scene...
///
///     // create the camera
///     commands
///         .spawn_bundle(Camera3dBundle {
///           transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
///           ..Default::default()
///         })
///         // IMPORTANT: add this component to your main camera
///         .insert(ColorBlindnessCamera);
/// }
/// ```
///
/// # Important note
///
/// This plugin only simulates how color blind players will see your game.
/// It does not correct for color blindness to make your game more accessible.
/// This plugin should only be used during development, and removed on final builds.
pub struct ColorBlindnessPlugin;
impl Plugin for ColorBlindnessPlugin {
    fn build(&self, app: &mut App) {
        let world = &mut app.world;
        world.get_resource_or_insert_with(ColorBlindnessParams::default);

        load_internal_asset!(
            app,
            COLOR_BLINDNESS_SHADER_HANDLE,
            "color_blindness.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(Material2dPlugin::<PostProcessingMaterial>::default())
            .add_startup_system(setup)
            .add_system(set_camera_target)
            .add_system(update_percentages);
    }
}

/// handle to the color blindness simulation shader
const COLOR_BLINDNESS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3937837360667146578);

/// Resource which selects the type of color blindness to simulate
///
/// # Example
///
/// This system will only enable the simulation while `Space` is held, and will cycle through
/// the different modes when `N` is pressed.
///
/// ```rust
/// # use bevy_app::prelude::*;
/// # use bevy_color_blindness::*;
/// # use bevy_ecs::prelude::*;
/// # use bevy_input::prelude::*;
/// # use bevy_math::prelude::*;
/// # use bevy_transform::prelude::*;
/// fn cycle_mode(input: Res<Input<KeyCode>>, mut params: ResMut<ColorBlindnessParams>) {
///     if input.just_pressed(KeyCode::N) {
///         params.mode.cycle();
///         println!("Changed to {:?}", params.mode);
///     }
///
///     params.enable = input.pressed(KeyCode::Space);
/// }
/// ```
#[derive(Default, Debug)]
pub struct ColorBlindnessParams {
    /// Selects the color blindness mode to use
    ///
    /// Defaults to `Mode::Normal`
    pub mode: Mode,
    /// Controls whether color blindness simulation is enabled
    ///
    /// Defaults to `false`
    pub enable: bool,
}

/// The different modes of color blindness simulation supported.
#[derive(Clone, Default, Debug)]
pub enum Mode {
    #[default]
    Normal,
    Protanopia,
    Protanomaly,
    Deuteranopia,
    Deuteranomaly,
    Tritanopia,
    Tritanomaly,
    Achromatopsia,
    Achromatomaly,
}

impl Mode {
    fn percentages(&self) -> (Vec3, Vec3, Vec3) {
        // table from https://www.alanzucconi.com/2015/12/16/color-blindness/
        // https://web.archive.org/web/20081014161121/http://www.colorjack.com/labs/colormatrix/

        match self {
            Mode::Normal => (Vec3::X, Vec3::Y, Vec3::Z),
            Mode::Protanopia => (
                [0.56667, 0.43333, 0.0].into(),
                [0.55833, 0.44167, 0.0].into(),
                [0.0, 0.24167, 0.75833].into(),
            ),
            Mode::Protanomaly => (
                [0.81667, 0.18333, 0.0].into(),
                [0.33333, 0.66667, 0.0].into(),
                [0.0, 0.125, 0.875].into(),
            ),
            Mode::Deuteranopia => (
                [0.625, 0.375, 0.0].into(),
                [0.70, 0.30, 0.0].into(),
                [0.0, 0.30, 0.70].into(),
            ),
            Mode::Deuteranomaly => (
                [0.80, 0.20, 0.0].into(),
                [0.25833, 0.74167, 0.0].into(),
                [0.0, 0.14167, 0.85833].into(),
            ),
            Mode::Tritanopia => (
                [0.95, 0.5, 0.0].into(),
                [0.0, 0.43333, 0.56667].into(),
                [0.0, 0.475, 0.525].into(),
            ),
            Mode::Tritanomaly => (
                [0.96667, 0.3333, 0.0].into(),
                [0.0, 0.73333, 0.26667].into(),
                [0.0, 0.18333, 0.81667].into(),
            ),
            Mode::Achromatopsia => (
                [0.299, 0.587, 0.114].into(),
                [0.299, 0.587, 0.114].into(),
                [0.299, 0.587, 0.114].into(),
            ),
            Mode::Achromatomaly => (
                [0.618, 0.32, 0.62].into(),
                [0.163, 0.775, 0.62].into(),
                [0.163, 0.320, 0.516].into(),
            ),
        }
    }

    /// Changes `self` to the next Mode.
    ///
    /// Useful for writing something like the following:
    ///
    /// ```rust
    /// # use bevy_app::prelude::*;
    /// # use bevy_color_blindness::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_input::prelude::*;
    /// fn cycle_mode(input: Res<Input<KeyCode>>, mut params: ResMut<ColorBlindnessParams>) {
    ///     if input.just_pressed(KeyCode::N) {
    ///         params.mode.cycle();
    ///         println!("Changed to {:?}", params.mode);
    ///     }
    /// }
    /// ```
    pub fn cycle(&mut self) {
        *self = match self {
            Mode::Normal => Mode::Protanopia,
            Mode::Protanopia => Mode::Protanomaly,
            Mode::Protanomaly => Mode::Deuteranopia,
            Mode::Deuteranopia => Mode::Deuteranomaly,
            Mode::Deuteranomaly => Mode::Tritanopia,
            Mode::Tritanopia => Mode::Tritanomaly,
            Mode::Tritanomaly => Mode::Achromatopsia,
            Mode::Achromatopsia => Mode::Achromatomaly,
            Mode::Achromatomaly => Mode::Normal,
        };
    }
}

/// Our custom post processing material
#[derive(AsBindGroup, TypeUuid, Clone)]
#[uuid = "bc2f08eb-a0fb-43f1-a908-54871ea597d5"]
struct PostProcessingMaterial {
    /// In this example, this image will be the result of the main camera.
    #[texture(0)]
    #[sampler(1)]
    source_image: Handle<Image>,

    #[uniform(2)]
    red: Vec3,
    #[uniform(2)]
    green: Vec3,
    #[uniform(2)]
    blue: Vec3,
}

impl Material2d for PostProcessingMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(COLOR_BLINDNESS_SHADER_HANDLE.typed())
    }
}

/// Component to identify your main camera
///
/// Adding this component to a camera will set up the post-processing pipeline
/// which simulates color blindness. This is done by changing the render target
/// to be an image, and then using another camera to render that image.
///
/// Cameras with `ColorBlindnessCamera` will have [`UiCameraConfig`] inserted with
/// `show_ui` set to `false`. This is to ensure that UI elements are not rendered twice.
/// In most cases, you will want to render UI using the final post-processing camera.
/// If for some reason this behavior is not desired, please open an issue.
///
/// [`UiCameraConfig`]: bevy_ui::entity::UiCameraConfig
#[derive(Component)]
pub struct ColorBlindnessCamera;

/// sets the target for newly added `ColorBlindCamera`s
fn set_camera_target(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Camera), Added<ColorBlindnessCamera>>,
    inner: Res<InternalResource>,
) {
    for (entity, mut camera) in query.iter_mut() {
        camera.target = RenderTarget::Image(inner.image.clone());
        commands
            .entity(entity)
            .insert(UiCameraConfig { show_ui: false });
    }
}

/// updates the percentages in the post processing material when the Mode changes in Params
fn update_percentages(
    params: Res<ColorBlindnessParams>,
    inner: Res<InternalResource>,
    mut materials: ResMut<Assets<PostProcessingMaterial>>,
) {
    if params.is_changed() {
        let mut mat = materials.get_mut(&inner.post).unwrap();

        let mode = if params.enable {
            &params.mode
        } else {
            &Mode::Normal
        };
        let (red, green, blue) = mode.percentages();

        mat.red = red;
        mat.green = green;
        mat.blue = blue;
    }
}

/// internal resource which holds the handles
struct InternalResource {
    image: Handle<Image>,
    post: Handle<PostProcessingMaterial>,
}

/// creates the image, the material, the final camera, and the whole post-processing pipeline
///
/// based on the post-processing example
/// `https://github.com/bevyengine/bevy/blob/main/examples/shader/post_processing.rs`
fn setup(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    params: Res<ColorBlindnessParams>,
) {
    asset_server.watch_for_changes().unwrap();

    let window = windows.get_primary_mut().unwrap();
    let size = Extent3d {
        width: window.physical_width(),
        height: window.physical_height(),
        ..Default::default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..Default::default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);

    // This specifies the layer used for the post processing camera, which will be attached to the post processing camera and 2d quad.
    let post_processing_pass_layer = RenderLayers::layer((RenderLayers::TOTAL_LAYERS - 1) as u8);

    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        size.width as f32,
        size.height as f32,
    ))));

    // This material has the texture that has been rendered.
    let (red, green, blue) = params.mode.percentages();
    let material_handle = post_processing_materials.add(PostProcessingMaterial {
        source_image: image_handle.clone(),
        red,
        green,
        blue,
    });

    commands.insert_resource(InternalResource {
        image: image_handle,
        post: material_handle.clone(),
    });

    // Post processing 2d quad, with material using the render texture done by the main camera, with a custom shader.
    commands
        .spawn_bundle(MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 1.5),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(post_processing_pass_layer);

    // The post-processing pass camera.
    commands
        .spawn_bundle(Camera2dBundle {
            camera: Camera {
                // renders after the first main camera which has default value: 0.
                priority: 1,
                ..Default::default()
            },
            ..Camera2dBundle::default()
        })
        .insert(post_processing_pass_layer);
}
