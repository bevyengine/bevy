//! This example demonstrates how to save assets in the common case where the asset contains no
//! subassets.

use bevy::{
    asset::{
        saver::{save_using_saver, SavedAsset},
        RenderAssetUsages,
    },
    camera::ScalingMode,
    color::palettes::tailwind,
    image::{ImageLoaderSettings, ImageSampler, ImageSaver, ImageSaverSettings},
    input::common_conditions::input_just_pressed,
    picking::pointer::Location,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::Anchor,
    tasks::IoTaskPool,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // This is just overriding the default asset paths to scope this to the correct example
            // folder. You can generally skip this in your own projects.
            file_path: "examples/asset/saved_assets".to_string(),
            ..Default::default()
        }))
        .add_plugins(image_drawing_plugin)
        .add_systems(
            PreUpdate,
            perform_save.run_if(input_just_pressed(KeyCode::F5)),
        )
        .run();
}

const ASSET_PATH: &str = "art_project.png";

fn perform_save(
    image_to_save: Res<ImageToSave>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let image = images.get(&image_to_save.0).unwrap();

    let image = image.clone();
    let asset_server = asset_server.clone();
    IoTaskPool::get()
        .spawn(async move {
            match save_using_saver(
                asset_server.clone(),
                &ImageSaver,
                &ASSET_PATH.into(),
                SavedAsset::from_asset(&image),
                &ImageSaverSettings::default(),
            )
            .await
            {
                Ok(()) => info!("Completed save of {ASSET_PATH}"),
                Err(err) => error!("Failed to save asset: {err}"),
            }
        })
        .detach();
}

/// Plugin for doing image drawing.
///
/// This doesn't really have anything to do with asset saving, but provides a real-use case.
fn image_drawing_plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_observer(on_drag_start)
        .add_observer(on_drag)
        .add_observer(try_plot)
        .init_resource::<DrawColor>()
        .add_observer(on_enter_selectable)
        .add_observer(on_leave_selectable)
        .add_observer(on_press_selectable);
}

#[derive(Resource)]
struct ImageToSave(Handle<Image>);

#[derive(Component)]
struct SpriteToSave;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 125.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    commands.spawn(Text(
        r"Select a color from the palette at the bottom
LMB - Draw with selected color
F5 - Save image"
            .into(),
    ));

    let handle = asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        })
        .load(ASSET_PATH);
    commands.spawn((
        Sprite {
            image: handle.clone(),
            ..Default::default()
        },
        SpriteToSave,
        Pickable::default(),
    ));

    // We're doing something a little cursed here: we initiate a load, and then insert a default
    // image into that handle. If the load succeeds, the image will be replaced with the loaded
    // contents. If it fails, the default image will remain. In real code, you likely want to poll
    // `AssetServer::load_state` and only insert this on load failure.
    images
        .insert(&handle, {
            let mut image = Image::new_fill(
                Extent3d {
                    width: 100,
                    height: 100,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 255],
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::all(),
            );
            image.sampler = ImageSampler::nearest();
            image
        })
        .unwrap();

    commands.insert_resource(ImageToSave(handle));

    let container = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::End,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            Pickable::IGNORE,
        ))
        .id();

    for color in [
        Color::WHITE,
        Color::Srgba(tailwind::RED_500),
        Color::Srgba(tailwind::ORANGE_500),
        Color::Srgba(tailwind::YELLOW_500),
        Color::Srgba(tailwind::GREEN_500),
        Color::Srgba(tailwind::BLUE_500),
        Color::Srgba(tailwind::INDIGO_500),
        Color::Srgba(tailwind::VIOLET_500),
        Color::BLACK,
    ] {
        let mut entity = commands.spawn((
            Node {
                width: vw(5),
                height: vh(5),
                border: px(5).all(),
                ..Default::default()
            },
            SelectableColor,
            BackgroundColor(color),
            BorderColor::all(NORMAL_COLOR),
            ChildOf(container),
        ));
        if color == Color::WHITE {
            entity.insert((Selected, BorderColor::all(SELECTED_COLOR)));
        }
    }
}

#[derive(EntityEvent)]
struct TryPlot {
    entity: Entity,
    location: Location,
}

fn on_drag_start(event: On<Pointer<DragStart>>, mut commands: Commands) {
    commands.trigger(TryPlot {
        entity: event.entity,
        location: event.pointer_location.clone(),
    });
}

fn on_drag(event: On<Pointer<Drag>>, mut commands: Commands) {
    commands.trigger(TryPlot {
        entity: event.entity,
        location: event.pointer_location.clone(),
    });
}

fn try_plot(
    event: On<TryPlot>,
    sprite: Query<(&Sprite, &Anchor, &GlobalTransform), With<SpriteToSave>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    texture_atlases: Res<Assets<TextureAtlasLayout>>,
    draw_color: Res<DrawColor>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok((sprite, anchor, sprite_transform)) = sprite.get(event.entity) else {
        return;
    };
    let (camera, camera_transform) = camera.into_inner();
    let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, event.location.position)
    else {
        return;
    };
    let relative_to_sprite = sprite_transform
        .affine()
        .inverse()
        .transform_point3(world_position.extend(0.0));
    let Ok(pixel_space) = sprite.compute_pixel_space_point(
        relative_to_sprite.xy(),
        *anchor,
        &images,
        &texture_atlases,
    ) else {
        return;
    };
    let pixel_coordinates = pixel_space.floor().as_uvec2();
    let mut image = images.get_mut(&sprite.image).unwrap();
    // For an actual drawing app, you'd at least draw a line from the last point, but this is
    // simpler.
    image
        .set_color_at(pixel_coordinates.x, pixel_coordinates.y, draw_color.0)
        .unwrap();
}

#[derive(Resource, Default)]
struct DrawColor(Color);

#[derive(Component)]
struct SelectableColor;

#[derive(Component)]
struct Selected;

const NORMAL_COLOR: Color = Color::BLACK;
const HIGHLIGHT_COLOR: Color = Color::Srgba(tailwind::NEUTRAL_500);
const SELECTED_COLOR: Color = Color::Srgba(tailwind::RED_600);

fn on_enter_selectable(
    event: On<Pointer<Enter>>,
    mut border: Query<&mut BorderColor, (With<SelectableColor>, Without<Selected>)>,
) {
    let Ok(mut border) = border.get_mut(event.entity) else {
        return;
    };

    *border = BorderColor::all(HIGHLIGHT_COLOR);
}

fn on_leave_selectable(
    event: On<Pointer<Leave>>,
    mut border: Query<&mut BorderColor, (With<SelectableColor>, Without<Selected>)>,
) {
    let Ok(mut border) = border.get_mut(event.entity) else {
        return;
    };

    *border = BorderColor::all(NORMAL_COLOR);
}

fn on_press_selectable(
    event: On<Pointer<Press>>,
    mut borders: Query<(Entity, &mut BorderColor, &BackgroundColor), With<SelectableColor>>,
    mut draw_color: ResMut<DrawColor>,
    mut commands: Commands,
) {
    if !borders.contains(event.entity) {
        return;
    }
    for (entity, mut border, _) in borders.iter_mut() {
        commands.entity(entity).remove::<Selected>();
        *border = BorderColor::all(NORMAL_COLOR);
    }
    let (_, mut border, background_color) = borders.get_mut(event.entity).unwrap();
    *border = BorderColor::all(SELECTED_COLOR);
    commands.entity(event.entity).insert(Selected);

    draw_color.0 = background_color.0;
}
