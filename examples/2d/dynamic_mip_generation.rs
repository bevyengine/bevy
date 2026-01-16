//! Demonstrates use of the mipmap generation plugin to generate mipmaps for a
//! texture.
//!
//! This example demonstrates use of the [`MipGenerationJobs`] resource to
//! generate mipmap levels for a texture at runtime. It generates the first
//! mipmap level of a texture on CPU, which consists of two ellipses with
//! randomly chosen colors. Then it invokes Bevy's mipmap generation pass to
//! generate the remaining mipmap levels for the texture on the GPU. You can use
//! the UI to regenerate the texture and adjust its size to prove that the
//! texture, and its mipmaps, are truly being generated at runtime and aren't
//! being built ahead of time.

use std::array;

use bevy::asset::RenderAssetTransferPriority;
use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::mip_generation::{MipGenerationJobs, MipGenerationNode, MipGenerationPhaseId},
    prelude::*,
    reflect::TypePath,
    render::{
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages},
        Extract, RenderApp,
    },
    shader::ShaderRef,
    sprite::Text2dShadow,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
    window::{PrimaryWindow, WindowResized},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::widgets::{
    RadioButton, RadioButtonText, WidgetClickEvent, WidgetClickSender, BUTTON_BORDER,
    BUTTON_BORDER_COLOR, BUTTON_BORDER_RADIUS_SIZE, BUTTON_PADDING,
};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// The time in seconds that it takes the animation of the image shrinking and
/// growing to play.
const ANIMATION_PERIOD: f32 = 2.0;

/// The path to the single mip level 2D material shader inside the `assets`
/// directory.
const SINGLE_MIP_LEVEL_SHADER_ASSET_PATH: &str = "shaders/single_mip_level.wgsl";

/// The distance from the left side of the column of mipmap slices to the right
/// side of the area used for the animation.
const MIP_SLICES_MARGIN_LEFT: f32 = 64.0;
/// The distance from the right side of the window to the right side of the
/// column of mipmap slices.
const MIP_SLICES_MARGIN_RIGHT: f32 = 12.0;
/// The width of the column of mipmap slices, not counting the labels, as a
/// fraction of the width of the window.
const MIP_SLICES_WIDTH: f32 = 1.0 / 6.0;

/// The size of the mipmap level label font.
const FONT_SIZE: f32 = 16.0;

/// All settings that the user can change via the UI.
#[derive(Resource)]
struct AppStatus {
    /// Whether mipmaps are to be generated for the image.
    enable_mip_generation: EnableMipGeneration,
    /// The width of the image.
    image_width: ImageSize,
    /// The height of the image.
    image_height: ImageSize,
    /// Seeded random generator.
    rng: ChaCha8Rng,
}

impl Default for AppStatus {
    fn default() -> Self {
        AppStatus {
            enable_mip_generation: EnableMipGeneration::On,
            image_width: ImageSize::Size640,
            image_height: ImageSize::Size480,
            rng: ChaCha8Rng::seed_from_u64(19878367467713),
        }
    }
}

/// Identifies one of the settings that can be changed by the user.
#[derive(Clone)]
enum AppSetting {
    /// Regenerates the top mipmap level.
    ///
    /// This is more of an *operation* than a *setting* per se, but it was
    /// convenient to use the `AppSetting` infrastructure for the "Regenerate
    /// Top Mip Level" button.
    RegenerateTopMipLevel,

    /// Whether mipmaps should be generated.
    EnableMipGeneration(EnableMipGeneration),

    /// The width of the image.
    ImageWidth(ImageSize),

    /// The height of the image.
    ImageHeight(ImageSize),
}

/// Whether mipmap levels will be generated.
///
/// Turning off the generation of mipmap levels, and then regenerating the
/// image, will cause all mipmap levels other than the first to be blank. This
/// will in turn cause the image to fade out as it shrinks, as the GPU switches
/// to rendering mipmap levels that don't have associated images.
#[derive(Clone, Copy, Default, PartialEq)]
enum EnableMipGeneration {
    /// Mipmap levels are generated for the image.
    #[default]
    On,
    /// Mipmap levels aren't generated for the image.
    Off,
}

/// Possible lengths for an image side from which the user can choose.
#[derive(Clone, Copy, Default, PartialEq)]
#[repr(u32)]
enum ImageSize {
    /// 240px.
    Size240 = 240,
    /// 480px (the default height).
    Size480 = 480,
    /// 640px (the default width).
    #[default]
    Size640 = 640,
    /// 1080px.
    Size1080 = 1080,
    /// 1920px.
    Size1920 = 1920,
}

/// A 2D material that displays only one mipmap level of a texture.
///
/// This is the material used for the column of mip levels on the right side of
/// the window.
#[derive(Clone, Asset, TypePath, AsBindGroup, Debug)]
struct SingleMipLevelMaterial {
    /// The mip level that this material will show, starting from 0.
    #[uniform(0)]
    mip_level: u32,
    /// The image that is to be shown.
    #[texture(1)]
    #[sampler(2)]
    texture: Handle<Image>,
}

impl Material2d for SingleMipLevelMaterial {
    fn fragment_shader() -> ShaderRef {
        SINGLE_MIP_LEVEL_SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// A marker component for the image on the left side of the window.
///
/// This is the image that grows and shrinks to demonstrate the effect of mip
/// levels' presence and absence.
#[derive(Component)]
struct AnimatedImage;

/// A resource that stores the main image for which mipmaps are to be generated
/// (or not generated, depending on the application settings).
#[derive(Resource, Deref, DerefMut)]
struct MipmapSourceImage(Handle<Image>);

/// An iterator that yields the size of each mipmap level for an image, one
/// after another.
struct MipmapSizeIterator {
    /// The size of the previous mipmap level, or `None` if this iterator is
    /// finished.
    size: Option<UVec2>,
}

/// A [`RenderLabel`] for the mipmap generation render node.
///
/// This is needed in order to order the mipmap generation node relative to the
/// node that renders the image for which mipmaps have been generated.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, RenderLabel)]
struct MipGenerationLabel;

/// A marker component for every mesh that displays the image.
///
/// When the image is regenerated, we despawn and respawn all entities with this
/// component.
#[derive(Component)]
struct ImageView;

/// A message that's sent whenever the image and the corresponding views need to
/// be regenerated.
#[derive(Clone, Copy, Debug, Message)]
struct RegenerateImage;

/// The application entry point.
fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Dynamic Mipmap Generation Example".into(),
                ..default()
            }),
            ..default()
        }),
        Material2dPlugin::<SingleMipLevelMaterial>::default(),
    ))
    .init_resource::<AppStatus>()
    .init_resource::<AppAssets>()
    .add_message::<RegenerateImage>()
    .add_message::<WidgetClickEvent<AppSetting>>()
    .add_systems(Startup, setup)
    .add_systems(Update, animate_image_scale)
    .add_systems(
        Update,
        (
            widgets::handle_ui_interactions::<AppSetting>,
            update_radio_buttons,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (handle_window_resize_events, regenerate_image_when_requested).chain(),
    )
    .add_systems(
        Update,
        handle_app_setting_change
            .after(widgets::handle_ui_interactions::<AppSetting>)
            .before(regenerate_image_when_requested),
    );

    // Because `MipGenerationJobs` is part of the render app, we need to add the
    // associated systems to that app, not the main one.

    let render_app = app.get_sub_app_mut(RenderApp).expect("Need a render app");

    // Add a `MipGenerationNode` corresponding to our phase to the render graph.
    let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
    render_graph.add_node(
        MipGenerationLabel,
        MipGenerationNode(MipGenerationPhaseId(0)),
    );

    // Add an edge so that our mip generation node will run prior to rendering
    // any cameras.
    // If your mip generation node needs to run before some cameras and after
    // others, you can use more complex constraints. Or, for more exotic
    // scenarios, you can also create a custom render node that wraps a
    // `MipGenerationNode` and examines properties of the camera to invoke the
    // node at the appropriate time.
    render_graph.add_node_edge(MipGenerationLabel, CameraDriverLabel);

    // Add the system that adds the image into the `MipGenerationJobs` list.
    // Note that this must run as part of the extract schedule, because it needs
    // access to resources from both the main world and the render world.
    render_app.add_systems(ExtractSchedule, extract_mipmap_source_image);

    app.run();
}

/// Global assets used for this example.
#[derive(Resource)]
struct AppAssets {
    /// A 2D rectangle mesh, used to display the individual images.
    rectangle: Handle<Mesh>,
    /// The font used to display the mipmap level labels on the right side of
    /// the window.
    text_font: TextFont,
}

impl FromWorld for AppAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let rectangle = meshes.add(Rectangle::default());

        let asset_server = world.resource::<AssetServer>();
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        let text_font = TextFont {
            font: font.into(),
            font_size: FONT_SIZE,
            ..default()
        };

        AppAssets {
            rectangle,
            text_font,
        }
    }
}

/// Spawns all the objects in the scene and creates the initial image and
/// associated resources.
fn setup(
    mut commands: Commands,
    mut regenerate_image_message_writer: MessageWriter<RegenerateImage>,
) {
    // Spawn the camera.
    commands.spawn(Camera2d);

    // Spawn the UI widgets at the bottom of the window.
    spawn_ui(&mut commands);

    // Schedule the image to be generated.
    regenerate_image_message_writer.write(RegenerateImage);
}

/// Spawns the UI widgets at the bottom of the window.
fn spawn_ui(commands: &mut Commands) {
    commands.spawn((
        widgets::main_ui_node(),
        children![
            // Spawn the "Regenerate Top Mip Level" button.
            (
                Button,
                Node {
                    border: BUTTON_BORDER,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: BUTTON_PADDING,
                    border_radius: BorderRadius::all(BUTTON_BORDER_RADIUS_SIZE),
                    ..default()
                },
                BUTTON_BORDER_COLOR,
                BackgroundColor(Color::BLACK),
                WidgetClickSender(AppSetting::RegenerateTopMipLevel),
                children![(
                    widgets::ui_text("Regenerate Top Mip Level", Color::WHITE),
                    WidgetClickSender(AppSetting::RegenerateTopMipLevel),
                )],
            ),
            // Spawn the "Mip Generation" switch that allows the user to toggle
            // mip generation on and off.
            widgets::option_buttons(
                "Mip Generation",
                &[
                    (
                        AppSetting::EnableMipGeneration(EnableMipGeneration::On),
                        "On"
                    ),
                    (
                        AppSetting::EnableMipGeneration(EnableMipGeneration::Off),
                        "Off"
                    ),
                ]
            ),
            // Spawn the "Image Width" control that allows the user to set the
            // width of the image.
            widgets::option_buttons(
                "Image Width",
                &[
                    (AppSetting::ImageWidth(ImageSize::Size240), "240"),
                    (AppSetting::ImageWidth(ImageSize::Size480), "480"),
                    (AppSetting::ImageWidth(ImageSize::Size640), "640"),
                    (AppSetting::ImageWidth(ImageSize::Size1080), "1080"),
                    (AppSetting::ImageWidth(ImageSize::Size1920), "1920"),
                ]
            ),
            // Spawn the "Image Height" control that allows the user to set the
            // height of the image.
            widgets::option_buttons(
                "Image Height",
                &[
                    (AppSetting::ImageHeight(ImageSize::Size240), "240"),
                    (AppSetting::ImageHeight(ImageSize::Size480), "480"),
                    (AppSetting::ImageHeight(ImageSize::Size640), "640"),
                    (AppSetting::ImageHeight(ImageSize::Size1080), "1080"),
                    (AppSetting::ImageHeight(ImageSize::Size1920), "1920"),
                ]
            ),
        ],
    ));
}

impl MipmapSizeIterator {
    /// Creates a [`MipmapSizeIterator`] corresponding to the size of the image
    /// currently being displayed.
    fn new(app_status: &AppStatus) -> MipmapSizeIterator {
        MipmapSizeIterator {
            size: Some(app_status.image_size_u32()),
        }
    }
}

impl Iterator for MipmapSizeIterator {
    type Item = UVec2;

    fn next(&mut self) -> Option<Self::Item> {
        // The size of mipmap level N + 1 is equal to half the size of mipmap
        // level N, rounding down, except that the size can never go below 1
        // pixel on either axis.
        let result = self.size;
        if let Some(size) = self.size {
            self.size = if size == UVec2::splat(1) {
                None
            } else {
                Some((size / 2).max(UVec2::splat(1)))
            };
        }
        result
    }
}

/// Updates the size of the image on the left side of the window each frame.
///
/// Resizing the image every frame effectively cycles through all the image's
/// mipmap levels, demonstrating the difference between the presence of mipmap
/// levels and their absence.
fn animate_image_scale(
    mut animated_images_query: Query<&mut Transform, With<AnimatedImage>>,
    windows_query: Query<&Window, With<PrimaryWindow>>,
    app_status: Res<AppStatus>,
    time: Res<Time>,
) {
    let window_size = windows_query.iter().next().unwrap().size();
    let animated_mesh_size = app_status.animated_mesh_size(window_size);

    for mut animated_image_transform in &mut animated_images_query {
        animated_image_transform.scale =
            animated_mesh_size.extend(1.0) * triangle_wave(time.elapsed_secs(), ANIMATION_PERIOD);
    }
}

/// Evaluates a [triangle wave] with the given wavelength.
///
/// This is used as part of [`animate_image_scale`], to derive the scale from
/// the current elapsed time.
///
/// [triangle wave]: https://en.wikipedia.org/wiki/Triangle_wave#Definition
fn triangle_wave(time: f32, wavelength: f32) -> f32 {
    2.0 * ops::abs(time / wavelength - ops::floor(time / wavelength + 0.5))
}

/// Adds the top mipmap level of the image to [`MipGenerationJobs`].
///
/// Note that this must run in the render world, not the main world, as
/// [`MipGenerationJobs`] is a resource that exists in the former. Consequently,
/// it must use [`Extract`] to access main world resources.
fn extract_mipmap_source_image(
    mipmap_source_image: Extract<Res<MipmapSourceImage>>,
    app_status: Extract<Res<AppStatus>>,
    mut mip_generation_jobs: ResMut<MipGenerationJobs>,
) {
    if app_status.enable_mip_generation == EnableMipGeneration::On {
        mip_generation_jobs.add(MipGenerationPhaseId(0), mipmap_source_image.id());
    }
}

/// Updates the widgets at the bottom of the screen to reflect the settings that
/// the user has chosen.
fn update_radio_buttons(
    mut widgets: Query<
        (
            Entity,
            Option<&mut BackgroundColor>,
            Has<Text>,
            &WidgetClickSender<AppSetting>,
        ),
        Or<(With<RadioButton>, With<RadioButtonText>)>,
    >,
    app_status: Res<AppStatus>,
    mut writer: TextUiWriter,
) {
    for (entity, image, has_text, sender) in widgets.iter_mut() {
        let selected = match **sender {
            AppSetting::RegenerateTopMipLevel => continue,
            AppSetting::EnableMipGeneration(enable_mip_generation) => {
                enable_mip_generation == app_status.enable_mip_generation
            }
            AppSetting::ImageWidth(image_width) => image_width == app_status.image_width,
            AppSetting::ImageHeight(image_height) => image_height == app_status.image_height,
        };

        if let Some(mut bg_color) = image {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }
}

/// Handles a request from the user to change application settings via the UI.
///
/// This also handles clicks on the "Regenerate Top Mip Level" button.
fn handle_app_setting_change(
    mut events: MessageReader<WidgetClickEvent<AppSetting>>,
    mut app_status: ResMut<AppStatus>,
    mut regenerate_image_message_writer: MessageWriter<RegenerateImage>,
) {
    for event in events.read() {
        // If this is a setting, update the setting. Fall through if, in
        // addition to updating the setting, we need to regenerate the image.
        match **event {
            AppSetting::EnableMipGeneration(enable_mip_generation) => {
                app_status.enable_mip_generation = enable_mip_generation;
                continue;
            }

            AppSetting::RegenerateTopMipLevel => {}
            AppSetting::ImageWidth(image_size) => app_status.image_width = image_size,
            AppSetting::ImageHeight(image_size) => app_status.image_height = image_size,
        }

        // Schedule the image to be regenerated.
        regenerate_image_message_writer.write(RegenerateImage);
    }
}

/// Handles resize events for the window.
///
/// Resizing the window invalidates the image and repositions all image views.
/// (Regenerating the image isn't strictly necessary, but it's simplest to have
/// a single function that both regenerates the image and recreates the image
/// views.)
fn handle_window_resize_events(
    mut events: MessageReader<WindowResized>,
    mut regenerate_image_message_writer: MessageWriter<RegenerateImage>,
) {
    for _ in events.read() {
        regenerate_image_message_writer.write(RegenerateImage);
    }
}

/// Recreates the image, as well as all views that show the image, when a
/// [`RegenerateImage`] message is received.
///
/// The views that show the image consist of the animated mesh on the left side
/// of the window and the column of mipmap level views on the right side of the
/// window.
fn regenerate_image_when_requested(
    mut commands: Commands,
    image_views_query: Query<Entity, With<ImageView>>,
    windows_query: Query<&Window, With<PrimaryWindow>>,
    app_assets: Res<AppAssets>,
    mut app_status: ResMut<AppStatus>,
    mut images: ResMut<Assets<Image>>,
    mut single_mip_level_materials: ResMut<Assets<SingleMipLevelMaterial>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut message_reader: MessageReader<RegenerateImage>,
) {
    // Only do this at most once per frame, or else the despawn logic below will
    // get confused.
    if message_reader.read().count() == 0 {
        return;
    }

    // Despawn all entities that show the image.
    for entity in image_views_query.iter() {
        commands.entity(entity).despawn();
    }

    // Regenerate the image.
    let image_handle = app_status.regenerate_mipmap_source_image(&mut commands, &mut images);

    // Respawn the animated image view on the left side of the window.
    spawn_animated_mesh(
        &mut commands,
        &app_status,
        &app_assets,
        &windows_query,
        &mut color_materials,
        &image_handle,
    );

    // Respawn the column of mip level views on the right side of the window.
    spawn_mip_level_views(
        &mut commands,
        &app_status,
        &app_assets,
        &windows_query,
        &mut single_mip_level_materials,
        &image_handle,
    );
}

/// Spawns the image on the left that continually changes scale.
///
/// Continually changing scale effectively cycles though each mip level,
/// demonstrating the difference between mip level images being present and mip
/// level image being absent.
fn spawn_animated_mesh(
    commands: &mut Commands,
    app_status: &AppStatus,
    app_assets: &AppAssets,
    windows_query: &Query<&Window, With<PrimaryWindow>>,
    color_materials: &mut Assets<ColorMaterial>,
    image_handle: &Handle<Image>,
) {
    let window_size = windows_query.iter().next().unwrap().size();
    let animated_mesh_area_size = app_status.animated_mesh_area_size(window_size);
    let animated_mesh_size = app_status.animated_mesh_size(window_size);

    commands.spawn((
        Mesh2d(app_assets.rectangle.clone()),
        MeshMaterial2d(color_materials.add(ColorMaterial {
            texture: Some(image_handle.clone()),
            ..default()
        })),
        Transform::from_translation(
            (animated_mesh_area_size * 0.5 - window_size * 0.5).extend(0.0),
        )
        .with_scale(animated_mesh_size.extend(1.0)),
        AnimatedImage,
        ImageView,
    ));
}

/// Creates the column on the right side of the window that displays each mip
/// level by itself.
fn spawn_mip_level_views(
    commands: &mut Commands,
    app_status: &AppStatus,
    app_assets: &AppAssets,
    windows_query: &Query<&Window, With<PrimaryWindow>>,
    single_mip_level_materials: &mut Assets<SingleMipLevelMaterial>,
    image_handle: &Handle<Image>,
) {
    let window_size = windows_query.iter().next().unwrap().size();

    // Calculate the placement of the column of mipmap levels.
    let max_slice_size = app_status.max_mip_slice_size(window_size);
    let y_origin = app_status.vertical_mip_slice_origin(window_size);
    let y_spacing = app_status.vertical_mip_slice_spacing(window_size);
    let x_origin = app_status.horizontal_mip_slice_origin(window_size);

    for (mip_level, mip_size) in MipmapSizeIterator::new(app_status).enumerate() {
        let y_center = y_origin - y_spacing * mip_level as f32;

        // Size each image to fit its container, preserving aspect ratio.
        let mut slice_size = mip_size.as_vec2();
        let ratios = max_slice_size / slice_size;
        let slice_scale = ratios.x.min(ratios.y).min(1.0);
        slice_size *= slice_scale;

        // Spawn the image. Use the `SingleMipLevelMaterial` with its custom
        // shader so that only the mip level in question is displayed.
        commands.spawn((
            Mesh2d(app_assets.rectangle.clone()),
            MeshMaterial2d(single_mip_level_materials.add(SingleMipLevelMaterial {
                mip_level: mip_level as u32,
                texture: image_handle.clone(),
            })),
            Transform::from_xyz(x_origin, y_center, 0.0).with_scale(slice_size.extend(1.0)),
            ImageView,
        ));

        // Display a label to the side.
        commands.spawn((
            Text2d::new(format!(
                "Level {}\n{}×{}",
                mip_level, mip_size.x, mip_size.y
            )),
            app_assets.text_font.clone(),
            TextLayout::new_with_justify(Justify::Center),
            Text2dShadow::default(),
            Transform::from_xyz(x_origin - max_slice_size.x * 0.5 - 64.0, y_center, 0.0),
            ImageView,
        ));
    }
}

/// Returns true if the given point is inside a 2D ellipse with the given center
/// and given radii or false otherwise.
fn point_in_ellipse(point: Vec2, center: Vec2, radii: Vec2) -> bool {
    // This can be derived from the standard equation of an ellipse:
    //
    //    x²   y²
    //    ⎯⎯ + ⎯⎯ = 1
    //    a²   b²
    let (nums, denoms) = (point - center, radii);
    let terms = (nums * nums) / (denoms * denoms);
    terms.x + terms.y < 1.0
}

impl AppStatus {
    /// Returns the vertical distance between each mip slice image in the column
    /// on the right side of the window.
    fn vertical_mip_slice_spacing(&self, window_size: Vec2) -> f32 {
        window_size.y / self.image_mip_level_count() as f32
    }

    /// Returns the Y position of the center of the image that represents the
    /// first mipmap level in the column on the right side of the window.
    fn vertical_mip_slice_origin(&self, window_size: Vec2) -> f32 {
        let spacing = self.vertical_mip_slice_spacing(window_size);
        window_size.y * 0.5 - spacing * 0.5
    }

    /// Returns the maximum area that a single mipmap slice can occupy in the
    /// column at the right side of the window.
    ///
    /// Because the slices may be smaller than this area, and because the size
    /// of each slice preserves the aspect ratio of the image, the actual
    /// displayed size of each slice may be smaller than this.
    fn max_mip_slice_size(&self, window_size: Vec2) -> Vec2 {
        let spacing = self.vertical_mip_slice_spacing(window_size);
        vec2(window_size.x * MIP_SLICES_WIDTH, spacing)
    }

    /// Returns the horizontal center point of each mip slice image in the
    /// column at the right side of the window.
    fn horizontal_mip_slice_origin(&self, window_size: Vec2) -> f32 {
        let max_slice_size = self.max_mip_slice_size(window_size);
        window_size.x * 0.5 - max_slice_size.x * 0.5 - MIP_SLICES_MARGIN_RIGHT
    }

    /// Calculates and returns the area reserved for the animated image on the
    /// left side of the window.
    ///
    /// Note that this isn't necessarily equal to the final size of the animated
    /// image, because that size preserves the image's aspect ratio.
    fn animated_mesh_area_size(&self, window_size: Vec2) -> Vec2 {
        vec2(
            self.horizontal_mip_slice_origin(window_size) * 2.0 - MIP_SLICES_MARGIN_LEFT * 2.0,
            window_size.y,
        )
    }

    /// Calculates and returns the actual maximum size of the animated image on
    /// the left side of the window.
    ///
    /// This is equal to the maximum portion of the
    /// [`Self::animated_mesh_area_size`] that the image can occupy while
    /// preserving its aspect ratio.
    fn animated_mesh_size(&self, window_size: Vec2) -> Vec2 {
        let max_image_size = self.animated_mesh_area_size(window_size);
        let image_size = self.image_size_f32();
        let ratios = max_image_size / image_size;
        let image_scale = ratios.x.min(ratios.y);
        image_size * image_scale
    }

    /// Returns the size of the image as a [`UVec2`].
    fn image_size_u32(&self) -> UVec2 {
        uvec2(self.image_width as u32, self.image_height as u32)
    }

    /// Returns the size of the image as a [`Vec2`].
    fn image_size_f32(&self) -> Vec2 {
        vec2(
            self.image_width as u32 as f32,
            self.image_height as u32 as f32,
        )
    }

    /// Regenerates the main image based on the image size selected by the user.
    fn regenerate_mipmap_source_image(
        &mut self,
        commands: &mut Commands,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        let image_data = self.generate_image_data();

        let mut image = Image::new_uninit(
            Extent3d {
                width: self.image_width as u32,
                height: self.image_height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::all(),
            RenderAssetTransferPriority::default(),
        );
        image.texture_descriptor.mip_level_count = self.image_mip_level_count();
        image.texture_descriptor.usage |= TextureUsages::STORAGE_BINDING;
        image.data = Some(image_data);

        let image_handle = images.add(image);
        commands.insert_resource(MipmapSourceImage(image_handle.clone()));

        image_handle
    }

    /// Draws the concentric ellipses that make up the image.
    ///
    /// Returns the RGBA8 image data.
    fn generate_image_data(&mut self) -> Vec<u8> {
        // Select random colors for the inner and outer ellipses.
        let outer_color: [u8; 3] = array::from_fn(|_| self.rng.random());
        let inner_color: [u8; 3] = array::from_fn(|_| self.rng.random());

        let image_byte_size = 4usize
            * MipmapSizeIterator::new(self)
                .map(|size| size.x as usize * size.y as usize)
                .sum::<usize>();
        let mut image_data = vec![0u8; image_byte_size];

        let center = self.image_size_f32() * 0.5;

        let inner_ellipse_radii = self.inner_ellipse_radii();
        let outer_ellipse_radii = self.outer_ellipse_radii();

        for y in 0..(self.image_height as u32) {
            for x in 0..(self.image_width as u32) {
                let p = vec2(x as f32, y as f32);
                let (color, alpha) = if point_in_ellipse(p, center, inner_ellipse_radii) {
                    (inner_color, 255)
                } else if point_in_ellipse(p, center, outer_ellipse_radii) {
                    (outer_color, 255)
                } else {
                    ([0; 3], 0)
                };
                let start = (4 * (x + y * (self.image_width as u32))) as usize;
                image_data[start..(start + 3)].copy_from_slice(&color);
                image_data[start + 3] = alpha;
            }
        }

        image_data
    }

    /// Returns the number of mipmap levels that the image should possess.
    ///
    /// This will be equal to the maximum number of mipmap levels that an image
    /// of the appropriate size can have.
    fn image_mip_level_count(&self) -> u32 {
        32 - (self.image_width as u32)
            .max(self.image_height as u32)
            .leading_zeros()
    }

    /// Returns the X and Y radii of the outer ellipse drawn in the texture,
    /// respectively.
    fn outer_ellipse_radii(&self) -> Vec2 {
        self.image_size_f32() * 0.5
    }

    /// Returns the X and Y radii of the inner ellipse drawn in the texture,
    /// respectively.
    fn inner_ellipse_radii(&self) -> Vec2 {
        self.image_size_f32() * 0.25
    }
}
