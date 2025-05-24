//! Display the Inventory of a specific Player in a Grid
//! using a custom Relationship to associate items (Entities)
//! with a Player
use bevy::{color::palettes::tailwind::*, ecs::spawn::SpawnIter, prelude::*};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() -> AppExit {
    App::new()
        .insert_resource(ClearColor(SLATE_950.into()))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (startup, show_inventory).chain())
        .add_observer(update_button_color_on::<Over>(SLATE_400.into()))
        .add_observer(update_button_color_on::<Out>(SLATE_500.into()))
        .add_observer(update_button_color_on::<Pressed>(GREEN_400.into()))
        .add_observer(update_button_color_on::<Released>(SLATE_500.into()))
        .run()
}

fn update_button_color_on<E>(
    color: Color,
) -> impl Fn(Trigger<Pointer<E>>, Query<&mut BackgroundColor, With<Button>>)
where
    E: std::fmt::Debug + Reflect + Clone,
{
    // An observer closure that accepts a Pointer Event and a Color. We do this to avoid needing to write four
    // versions of this observer, each triggered by a different event and with a different hardcoded
    // Event/Color. Instead, the event type is a generic, and the Event/Color is passed in.
    move |trigger: Trigger<Pointer<E>>, mut buttons: Query<&mut BackgroundColor, With<Button>>| {
        let Ok(mut background_color) = buttons.get_mut(trigger.target()) else {
            return;
        };

        background_color.0 = color;
    }
}

/// A custom Relationship that associates Items with
/// an Item Holder (aka an Inventory)
#[derive(Debug, Component)]
#[relationship(relationship_target = Inventory)]
struct ItemOf(Entity);

/// The collection of Items an entity is holding
#[derive(Component)]
#[relationship_target(relationship = ItemOf)]
struct Inventory(Vec<Entity>);

/// The texture atlas index for an item's
/// ImageNode display
#[derive(Debug, Component)]
struct ItemDisplayImage(usize);

/// A Resource that holds handles for the item textures
/// and the layout for the texture atlas for the items.
#[derive(Resource)]
struct ItemTextures {
    texture: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

/// A marker Component for the Player
#[derive(Component)]
struct Player;

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2d);

    // Use seeded rng and store it in a resource; this makes the random output reproducible.
    let mut seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);

    // generate a random set of a fake amount of items
    // for display purposes
    let items: Vec<usize> = (0..100)
        .into_iter()
        .map(|_| seeded_rng.gen_range(0..25))
        .collect();

    // build and store the
    let texture = asset_server.load("textures/food_kenney.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 7, 7, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands.insert_resource(ItemTextures {
        texture,
        layout: texture_atlas_layout,
    });

    // Spawn a player with a random selection of items
    commands.spawn((
        Name::new("Player"),
        Player,
        Inventory::spawn(SpawnIter(
            items.into_iter().map(|index| ItemDisplayImage(index)),
        )),
    ));
}

fn show_inventory(
    mut commands: Commands,
    inventory: Single<&Inventory, With<Player>>,
    item_textures: Res<ItemTextures>,
    items: Query<&ItemDisplayImage, With<ItemOf>>,
) {
    let texture = item_textures.texture.clone();
    let layout = item_textures.layout.clone();

    let items_to_render: Vec<_> = inventory
        .0
        .iter()
        .filter_map(|entity| items.get(*entity).ok().map(|v| v.0))
        .fold(vec![], |mut acc: Vec<(usize, usize)>, index: usize| {
            match acc.iter().position(|item| item.0 == index) {
                Some(position) => {
                    acc[position].1 += 1;
                }
                None => {
                    acc.push((index, 1));
                }
            }
            acc
        })
        .into_iter()
        .map(move |(atlas_index, count)| {
            (
                Button,
                BackgroundColor(SLATE_500.into()),
                Node {
                    display: Display::Grid,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::End,
                    ..default()
                },
                Outline::new(Val::Px(2.), Val::ZERO, SLATE_50.into()),
                ImageNode::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout.clone(),
                        index: atlas_index,
                    },
                ),
                children![(
                    Node {
                        width: Val::Px(15.),
                        height: Val::Px(15.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(SLATE_50.into()),
                    children![(
                        // Display the item count
                        Text::new(count.to_string()),
                        TextFont::from_font_size(16.0),
                        TextLayout::new_with_justify(JustifyText::Center),
                        TextColor(SLATE_950.into())
                    )]
                )],
            )
        })
        .collect();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            children![(
                Node {
                    // Use the CSS Grid algorithm for laying out this node
                    display: Display::Grid,
                    // Make the size of the inventory a specific size
                    width: Val::Px(250.0),
                    height: Val::Px(250.0),
                    // A gap between each grid item
                    row_gap: Val::Px(7.),
                    column_gap: Val::Px(7.),
                    // a 5x5 grid layout that will size the tracks as 1/5
                    // of the total grid size
                    grid_template_columns: RepeatedGridTrack::flex(5, 1.),
                    grid_template_rows: RepeatedGridTrack::flex(5, 1.),
                    ..default()
                },
                Children::spawn(SpawnIter(items_to_render.into_iter())),
            )],
        ))
        .observe(|trigger: Trigger<Pointer<Click>>| {
            info!(entity = ?trigger.event().target, "clicked");
        });
}
