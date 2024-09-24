//! Shows how to modify texture assets after spawning.

use bevy::{
    input::common_conditions::input_just_pressed,
    prelude::*,
    render::{render_asset::RenderAssetUsages, texture::ImageLoaderSettings},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, spawn_text))
        .add_systems(
            Update,
            alter_handle.run_if(input_just_pressed(KeyCode::Space)),
        )
        .add_systems(
            Update,
            alter_asset.run_if(input_just_pressed(KeyCode::Enter)),
        )
        .run();
}

#[derive(Component, Debug)]
enum Bird {
    Normal,
    Logo,
}

impl Bird {
    fn get_texture_path(&self) -> String {
        match self {
            Bird::Normal => "branding/bevy_bird_dark.png".into(),
            Bird::Logo => "branding/bevy_logo_dark.png".into(),
        }
    }

    fn set_next_variant(&mut self) {
        *self = match self {
            Bird::Normal => Bird::Logo,
            Bird::Logo => Bird::Normal,
        }
    }
}

#[derive(Component, Debug)]
struct Left;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bird_left = Bird::Normal;
    let bird_right = Bird::Normal;
    commands.spawn(Camera2dBundle::default());

    let texture_left = asset_server.load_with_settings(
        bird_left.get_texture_path(),
        // `RenderAssetUsages::all()` is already the default, so the line below could be omitted.
        // It's helpful to know it exists, however.
        //
        // `RenderAssetUsages` tell Bevy whether to keep the data around:
        //   - for the GPU (`RenderAssetUsages::RENDER_WORLD`),
        //   - for the CPU (`RenderAssetUsages::MAIN_WORLD`),
        //   - or both.
        // `RENDER_WORLD` is necessary to render the image, `MAIN_WORLD` is necessary to inspect
        // and modify the image (via `ResMut<Assets<Image>>`).
        //
        // Since most games will not need to modify textures at runtime, many developers opt to pass
        // only `RENDER_WORLD`. This is more memory efficient, as we don't need to keep the image in
        // RAM. For this example however, this would not work, as we need to inspect and modify the
        // image at runtime.
        |settings: &mut ImageLoaderSettings| settings.asset_usage = RenderAssetUsages::all(),
    );

    commands.spawn((
        Name::new("Bird Left"),
        // This marker component ensures we can easily find either of the Birds by using With and
        // Without query filters.
        Left,
        SpriteBundle {
            texture: texture_left,
            transform: Transform::from_xyz(-200.0, 0.0, 0.0),
            ..default()
        },
        bird_left,
    ));

    commands.spawn((
        Name::new("Bird Right"),
        SpriteBundle {
            // In contrast to the above, here we rely on the default `RenderAssetUsages` loader
            // setting.
            texture: asset_server.load(bird_right.get_texture_path()),
            transform: Transform::from_xyz(200.0, 0.0, 0.0),
            ..default()
        },
        bird_right,
    ));
}

fn spawn_text(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Instructions"),
            NodeBundle {
                style: Style {
                    align_items: AlignItems::Start,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Start,
                    width: Val::Percent(100.),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Space: swap image texture paths by mutating a Handle<Image>",
                TextStyle::default(),
            ));
            parent.spawn(TextBundle::from_section(
                "Return: mutate the image Asset itself, changing all copies of it",
                TextStyle::default(),
            ));
        });
}

fn alter_handle(
    asset_server: Res<AssetServer>,
    mut right_bird: Query<(&mut Bird, &mut Handle<Image>), Without<Left>>,
) {
    // Image handles, like other parts of the ECS, can be queried as mutable and modified at
    // runtime. We only spawned one bird without the `Left` marker component.
    let Ok((mut bird, mut handle)) = right_bird.get_single_mut() else {
        return;
    };

    // Switch to a new Bird variant
    bird.set_next_variant();

    // Modify the handle associated with the Bird on the right side. Note that we will only
    // have to load the same path from storage media once: repeated attempts will re-use the
    // asset.
    *handle = asset_server.load(bird.get_texture_path());
}

fn alter_asset(mut images: ResMut<Assets<Image>>, left_bird: Query<&Handle<Image>, With<Left>>) {
    // It's convenient to retrieve the asset handle stored with the bird on the left. However,
    // we could just as easily have retained this in a resource or a dedicated component.
    let Ok(handle) = left_bird.get_single() else {
        return;
    };

    // Obtain a mutable reference to the Image asset.
    let Some(image) = images.get_mut(handle) else {
        return;
    };

    for pixel in &mut image.data {
        // Directly modify the asset data, which will affect all users of this asset. By
        // contrast, mutating the handle (as we did above) affects only one copy. In this case,
        // we'll just invert the colors, by way of demonstration. Notice that both uses of the
        // asset show the change, not just the one on the left.
        *pixel = 255 - *pixel;
    }
}
