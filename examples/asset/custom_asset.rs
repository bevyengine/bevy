//! Demonstrates creating a simple data asset that can be loaded from disk.

use bevy::{
    asset::{common_loaders::ron::RonLoader, ReflectAsset},
    prelude::*,
};

// To create a custom asset, just derive the [`Asset`] trait. If you want to use this type with
// something like the [`RonLoader`] (seen below), you also need to derive [`Reflect`] and reflect
// [`Asset`].
#[derive(Asset, Reflect, Debug)]
#[reflect(Asset)]
struct MyDataAsset {
    text: String,
    color: Color,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Adding this RonLoader allows us to load any reflected type in the RON format! This is
        // **not** required (you can define your own asset loader as shown in the
        // `custom_asset_loader` example, or avoid loading assets altogether and just use
        // `Assets::add` instead), but this loader allows us to easily load our data from disk.
        .init_asset_loader::<RonLoader>()
        .init_asset::<MyDataAsset>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            // Run condition so that we only update when there's an asset event, and not every
            // frame.
            update_text.run_if(on_message::<AssetEvent<MyDataAsset>>),
        )
        .run();
}

/// Spawns some centered text that will be updated by the data asset.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    let parent = commands
        .spawn(Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        })
        .id();

    commands.spawn((
        // We load the asset from `assets/data/simple_data.ron`. Try updating the file and see the
        // text change! Consider enabling the `file_watcher` feature to see this asset get
        // hot-reloaded.
        UpdateTextFromDataAsset(asset_server.load("data/simple_data.ron#Typed")),
        Text(String::new()),
        TextFont {
            font_size: FontSize::Vh(10.0),
            ..Default::default()
        },
        ChildOf(parent),
    ));
}

#[derive(Component)]
struct UpdateTextFromDataAsset(Handle<MyDataAsset>);

fn update_text(
    mut texts: Populated<(&UpdateTextFromDataAsset, &mut Text, &mut TextColor)>,
    data_assets: Res<Assets<MyDataAsset>>,
) {
    // The way this is written, any time any `MyDataAsset` changes state (loads, gets removed, loads
    // its dependencies, etc), every text will be updated. Normally, you'd want to read the
    // AssetEvent messages and just update the entities using that one asset, but for this example
    // this is sufficient.
    for (data_asset, mut text, mut color) in texts.iter_mut() {
        let Some(data_asset) = data_assets.get(&data_asset.0) else {
            // Might not be loaded yet.
            continue;
        };
        text.0 = data_asset.text.clone();
        color.0 = data_asset.color;
    }
}
