//! This example demonstrates how to save assets.

use bevy::{
    asset::{
        io::{Reader, Writer},
        saver::{save_using_saver, AssetSaver, SavedAsset, SavedAssetBuilder},
        AssetLoader, AsyncWriteExt, LoadContext,
    },
    color::palettes::tailwind,
    input::common_conditions::input_just_pressed,
    prelude::*,
    tasks::IoTaskPool,
};
use serde::{Deserialize, Serialize};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // This is just overriding the default asset paths to scope this to the correct example
            // folder. You can generally skip this in your own projects.
            file_path: "examples/asset/saved_assets".to_string(),
            ..Default::default()
        }))
        .add_plugins(box_editing_plugin)
        .init_asset::<OneBox>()
        .init_asset::<ManyBoxes>()
        .register_asset_loader(ManyBoxesLoader)
        .add_systems(
            PreUpdate,
            (
                perform_save.run_if(input_just_pressed(KeyCode::F5)),
                (
                    start_load.run_if(input_just_pressed(KeyCode::F6)),
                    wait_for_pending_loads,
                )
                    .chain(),
            ),
        )
        .run();
}

const ASSET_PATH: &str = "my_scene.boxes";

/// A system that takes the scene data, passes it to a task, and saves that scene data to
/// [`ASSET_PATH`].
fn perform_save(boxes: Query<(&Sprite, &Transform), With<Box>>, asset_server: Res<AssetServer>) {
    // First we extract all the data needed to produce an asset we can save.
    let boxes = boxes
        .iter()
        .enumerate()
        .map(|(index, (sprite, transform))| {
            (
                index.to_string(),
                OneBox {
                    position: transform.translation.xy(),
                    color: sprite.color,
                },
            )
        })
        .collect::<Vec<_>>();

    let asset_server = asset_server.clone();
    IoTaskPool::get()
        .spawn(async move {
            // Build a `SavedAsset` instance from the boxes we extracted.
            let mut builder = SavedAssetBuilder::new(asset_server.clone(), ASSET_PATH.into());
            let mut many_boxes = ManyBoxes { boxes: vec![] };
            for (label, one_box) in boxes.iter() {
                many_boxes.boxes.push(
                    builder
                        .add_labeled_asset_with_new_handle(label, SavedAsset::from_asset(one_box)),
                );
            }

            let saved_asset = builder.build(&many_boxes);
            // Save the asset using the provided saver.
            match save_using_saver(
                asset_server.clone(),
                &ManyBoxesSaver,
                &ASSET_PATH.into(),
                saved_asset,
                &(),
            )
            .await
            {
                Ok(()) => info!("Completed save of {ASSET_PATH}"),
                Err(err) => error!("Failed to save asset: {err}"),
            }
        })
        .detach();
}

/// A system the starts loading [`ASSET_PATH`].
fn start_load(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(PendingLoad(asset_server.load(ASSET_PATH)));
}

/// Marks that a handle is currently loading.
///
/// Once loading is complete, the [`ManyBoxes`] data will be spawned.
#[derive(Component)]
struct PendingLoad(Handle<ManyBoxes>);

/// Waits for any [`PendingLoad`]s to complete, and spawns in their boxes when they do.
fn wait_for_pending_loads(
    loads: Populated<(Entity, &PendingLoad)>,
    many_boxes: Res<Assets<ManyBoxes>>,
    one_boxes: Res<Assets<OneBox>>,
    existing_boxes: Query<Entity, With<Box>>,
    mut commands: Commands,
) {
    for (entity, load) in loads.iter() {
        let Some(many_boxes) = many_boxes.get(&load.0) else {
            continue;
        };

        commands.entity(entity).despawn();
        for entity in existing_boxes.iter() {
            commands.entity(entity).despawn();
        }

        for box_handle in many_boxes.boxes.iter() {
            let Some(one_box) = one_boxes.get(box_handle) else {
                return;
            };
            commands.spawn((
                Sprite::from_color(one_box.color, Vec2::new(100.0, 100.0)),
                Transform::from_translation(one_box.position.extend(0.0)),
                Pickable::default(),
                Box,
            ));
        }
    }
}

/// An asset representing a single box.
#[derive(Asset, TypePath, Clone, Serialize, Deserialize)]
struct OneBox {
    /// The position of the box.
    position: Vec2,
    /// The color of the box.
    color: Color,
}

/// An asset representing many boxes.
#[derive(Asset, TypePath)]
struct ManyBoxes {
    /// Stores handles to all the boxes that should be spawned.
    ///
    /// Note: in this trivial example, it seems more reasonable to just store [`Vec<OneBox>`], but
    /// in a more realistic example this could be something like a whole [`Mesh`] (where a handle
    /// makes more sense). We use a handle here to demonstrate saving subassets as well.
    boxes: Vec<Handle<OneBox>>,
}

/// A serializable version of [`ManyBoxes`].
#[derive(Serialize, Deserialize)]
struct SerializableManyBoxes {
    /// The boxes that exist in this scene.
    boxes: Vec<OneBox>,
}

/// Am asset saver to save [`ManyBoxes`] assets.
#[derive(TypePath)]
struct ManyBoxesSaver;

impl AssetSaver for ManyBoxesSaver {
    type Asset = ManyBoxes;
    type Error = BevyError;
    type OutputLoader = ManyBoxesLoader;
    type Settings = ();

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, '_, Self::Asset>,
        _settings: &Self::Settings,
    ) -> Result<(), Self::Error> {
        let boxes = asset
            .boxes
            .iter()
            .map(|handle| {
                // TODO: We should have a better to get the asset for a subasset handle.
                let label = handle
                    .path()
                    .and_then(|path| path.label())
                    .ok_or_else(|| format!("Failed to get label for handle {handle:?}"))?;
                asset
                    .get_labeled::<OneBox>(label)
                    .map(|subasset| subasset.get().clone())
                    .ok_or_else(|| format!("Failed to find labeled asset for label {label}"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Note: serializing to string isn't ideal since we can't do a streaming write, but this is
        // fine for an example.
        let serialized = ron::to_string(&SerializableManyBoxes { boxes })?;
        writer.write_all(serialized.as_bytes()).await?;

        Ok(())
    }
}

/// An asset loader for loading [`ManyBoxes`] assets.
#[derive(TypePath)]
struct ManyBoxesLoader;

impl AssetLoader for ManyBoxesLoader {
    type Asset = ManyBoxes;
    type Error = BevyError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes).await?;

        let serialized: SerializableManyBoxes = ron::de::from_bytes(&bytes)?;

        // Add the boxes as subassets.
        let mut result_boxes = vec![];
        for (index, one_box) in serialized.boxes.into_iter().enumerate() {
            result_boxes.push(load_context.add_labeled_asset(index.to_string(), one_box));
        }

        Ok(ManyBoxes {
            boxes: result_boxes,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["boxes"]
    }
}

/// Plugin for doing all the box-editing.
///
/// This doesn't really have anything to do with asset saving, but provides a real use-case.
fn box_editing_plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_observer(spawn_box)
        .add_observer(start_rotate_box_hue)
        .add_observer(end_rotate_box_hue_on_release)
        .add_observer(end_rotate_box_hue_on_out)
        .add_systems(Update, rotate_hue)
        .add_observer(stop_propagate_on_clicked_box)
        .add_observer(drag_box);
}

#[derive(Component)]
struct Box;

/// Spawns the initial scene.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn(Text(
        r"LMB (on background) - spawn new box
LMB (on box) - drag to move
RMB (on box) - rotate colors
F5 - Save boxes
F6 - Load boxes"
            .into(),
    ));
}

/// Spawns a new box whenever you left-click on the background.
fn spawn_box(
    event: On<Pointer<Press>>,
    window: Query<(), With<Window>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    if !window.contains(event.entity) {
        return;
    }

    let (camera, camera_transform) = camera.into_inner();
    let Ok(click_point) =
        camera.viewport_to_world_2d(camera_transform, event.pointer_location.position)
    else {
        return;
    };
    commands.spawn((
        Sprite::from_color(tailwind::RED_500, Vec2::new(100.0, 100.0)),
        Transform::from_translation(click_point.extend(0.0)),
        Pickable::default(),
        Box,
    ));
}

/// A component to rotate the hue of a sprite every frame.
#[derive(Component)]
struct RotateHue;

/// Rotates the hue of each [`Sprite`] tagged with [`RotateHue`].
fn rotate_hue(time: Res<Time>, mut sprites: Query<&mut Sprite, With<RotateHue>>) {
    for mut sprite in sprites.iter_mut() {
        // Make a full rotation every 2 seconds.
        sprite.color = sprite.color.rotate_hue(time.delta_secs() * 180.0);
    }
}

/// Starts rotating the hue of a box that has been right-clicked.
fn start_rotate_box_hue(
    event: On<Pointer<Press>>,
    boxes: Query<(), With<Box>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Secondary {
        return;
    }
    if !boxes.contains(event.entity) {
        return;
    }
    commands.entity(event.entity).insert(RotateHue);
}

/// Stops rotating the box hue if it's right-click is released.
fn end_rotate_box_hue_on_release(
    event: On<Pointer<Release>>,
    boxes: Query<(), (With<Box>, With<RotateHue>)>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Secondary {
        return;
    }
    if !boxes.contains(event.entity) {
        return;
    }
    commands.entity(event.entity).remove::<RotateHue>();
}

/// Stops rotating the box hue if the cursor moves off the entity.
fn end_rotate_box_hue_on_out(
    event: On<Pointer<Out>>,
    boxes: Query<(), (With<Box>, With<RotateHue>)>,
    mut commands: Commands,
) {
    if !boxes.contains(event.entity) {
        return;
    }
    commands.entity(event.entity).remove::<RotateHue>();
}

/// Blocks propagation of pointer press events on left-clicked boxes.
fn stop_propagate_on_clicked_box(mut event: On<Pointer<Press>>, boxes: Query<(), With<Box>>) {
    if event.button != PointerButton::Primary {
        return;
    }
    if !boxes.contains(event.entity) {
        return;
    }
    event.propagate(false);
}

/// Drags a box when you left-click on one.
fn drag_box(event: On<Pointer<Drag>>, mut boxes: Query<&mut Transform, With<Box>>) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(mut transform) = boxes.get_mut(event.entity) else {
        return;
    };

    // This is wrong in general (e.g., doesn't consider scale), but it's close enough for our
    // purposes.
    transform.translation += Vec3::new(event.delta.x, -event.delta.y, 0.0);
}
