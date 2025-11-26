//! Shows different types of selection menu.
//!
//! [`SelectionMenu::Single`] displays all items in a single horizontal line.

use std::borrow::BorrowMut;

use bevy::{
    app::{App, PluginGroup, Startup, Update},
    asset::{AssetServer, Handle},
    color::{Alpha, Color},
    core_pipeline::core_2d::Camera2d,
    ecs::{
        bundle::Bundle,
        children,
        component::Component,
        entity::Entity,
        hierarchy::Children,
        lifecycle::{Insert, Replace},
        name::Name,
        observer::On,
        query::{Has, With},
        relationship::Relationship,
        schedule::{IntoScheduleConfigs, SystemCondition},
        spawn::{SpawnIter, SpawnRelated},
        system::{Commands, Query, Res, Single},
    },
    image::{
        Image, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor, TextureAtlas,
        TextureAtlasLayout,
    },
    input::{common_conditions::input_just_pressed, keyboard::KeyCode},
    math::{ops, UVec2},
    render::{
        camera::{OrthographicProjection, Projection, ScalingMode},
        texture::ImagePlugin,
        view::Visibility,
    },
    sprite::Sprite,
    state::{
        app::AppExtStates,
        commands::CommandsStatesExt,
        condition::in_state,
        state::{OnEnter, OnExit, States},
    },
    time::Time,
    transform::components::Transform,
    ui::{
        widget::ImageNode, BackgroundColor, FlexDirection, Node, PositionType, UiRect, Val, ZIndex,
    },
    DefaultPlugins,
};

/// How fast the ui background darkens/lightens
const DECAY_FACTOR: f32 = 0.875;
/// Target Ui Background Alpha
const DARK_UI_BACKGROUND: f32 = 0.75;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().set(ImagePlugin {
        default_sampler: ImageSamplerDescriptor::nearest(),
    }))
    .add_plugins(single::SingleSelectionMenuPlugin);

    // Init states used by the example.
    // `GameState` indicates if the game is in `Game`, or shown the `SelectionMenu`
    // `SelectionMenu` indicates the style of the `SelectionMenu` being shown
    app.init_state::<GameState>().init_state::<SelectionMenu>();

    // Show or hide the selection menu by using `Tab`
    app.add_systems(
        Update,
        show_selection_menu.run_if(in_state(GameState::Game).and(input_just_pressed(KeyCode::Tab))),
    )
    .add_systems(
        Update,
        hide_selection_menu
            .run_if(in_state(GameState::SelectionMenu).and(input_just_pressed(KeyCode::Tab))),
    );

    app
        // Initialize inventory
        .add_systems(Startup, fill_inventory)
        // Observers to present and remove items from the quick slot
        .add_observer(present_item)
        .add_observer(presented_item_lost)
        // Update Ui background's alpha
        .add_systems(OnEnter(GameState::SelectionMenu), darker_ui_background)
        .add_systems(OnExit(GameState::SelectionMenu), lighten_ui_background)
        .add_systems(Update, (update_ui_background, update_image_node_alpha));

    // For visuals
    app.add_systems(Startup, setup_world);

    app.run();
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
enum GameState {
    #[default]
    Game,
    SelectionMenu,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
enum SelectionMenu {
    #[default]
    Single,
    #[expect(dead_code, reason = "TODO")]
    Stacked,
}

fn show_selection_menu(mut commands: Commands) {
    commands.set_state(GameState::SelectionMenu);
}

fn hide_selection_menu(mut commands: Commands) {
    commands.set_state(GameState::Game);
}

fn present_item(
    trigger: On<Insert, PresentingItem>,
    mut commands: Commands,
    presenters: Query<&PresentingItem, With<Node>>,
    items: Query<&Sprite, With<ItemId>>,
) {
    let Ok(presenter) = presenters.get(trigger.target()) else {
        unreachable!("Entity must already have PresentingItem inside the Insert observer.");
    };
    let Ok(item) = items.get(presenter.get()) else {
        unreachable!("Tried to add an entity that was not an item to the quick slot");
    };
    commands.entity(trigger.target()).insert(ImageNode {
        image: item.image.clone(),
        texture_atlas: item.texture_atlas.clone(),
        ..Default::default()
    });
}

fn presented_item_lost(trigger: On<Replace, PresentingItem>, mut commands: Commands) {
    commands.entity(trigger.target()).despawn_children();
}

/// The list of items to show on the selection menu
#[derive(Debug, Component)]
struct Inventory;

/// An [`Item`] contains a [`Name`], [`Sprite`], [`ItemId`], [`ItemCategory`]
#[derive(Debug, Clone, Bundle)]
struct Item {
    name: Name,
    sprite: Sprite,
    item_id: ItemId,
    category: ItemCategory,
}

/// Unique item id
#[derive(Debug, Clone, Component)]
#[expect(dead_code, reason = "Will be used on sorting later")]
struct ItemId(u8);

/// The category that the item belongs to
#[derive(Debug, Clone, Component)]
enum ItemCategory {
    Fruit,
    Vegetable,
    Ingredient,
    Condiment,
    Protein,
    Soup,
    Canned,
    Hamburger,
    Cake,
    Chocolate,
    Tool,
    Liquid,
    Cheese,
}

/// Ui background marker
#[derive(Debug, Component)]
#[require(BackgroundColor = BackgroundColor(Color::BLACK.with_alpha(0.)))]
pub struct UiBackground;

/// Sets a target alpha an entity.
#[derive(Debug, Component)]
pub struct TargetAlpha(f32);

/// Marks an entity as having fixed alpha. This prevents it's alpha from being modified
/// even if [`TargetAlpha`] is added.
#[derive(Debug, Component)]
pub struct FixedAlpha;

/// Marker component for the quick slot ui
#[derive(Debug, Component)]
#[require(Node)]
pub struct QuickSlotUi;

/// Refers to the entity being displayed on this UI node
#[derive(Debug, Clone, Component)]
#[relationship(relationship_target = PresentedIn)]
pub struct PresentingItem(Entity);

/// Refers to the UI nodes this item is being presented on
#[derive(Debug, Component)]
#[relationship_target(relationship = PresentingItem)]
pub struct PresentedIn(Vec<Entity>);

mod single {
    use bevy::{
        app::{Plugin, Startup, Update},
        asset::AssetServer,
        color::Color,
        ecs::{
            children,
            component::Component,
            entity::Entity,
            hierarchy::Children,
            lifecycle::{Insert, Replace},
            name::Name,
            observer::On,
            query::{Has, With},
            schedule::IntoScheduleConfigs,
            spawn::SpawnIter,
            system::{Commands, Query, Res, Single},
        },
        input::{keyboard::KeyCode, ButtonInput},
        prelude::SpawnRelated,
        render::view::Visibility,
        state::{
            app::AppExtStates,
            condition::in_state,
            state::{ComputedStates, OnEnter, OnExit},
        },
        text::{TextColor, TextFont},
        time::Time,
        ui::{
            widget::{ImageNode, Text},
            AlignItems, AlignSelf, FlexDirection, JustifyContent, Node, Overflow, PositionType,
            ScrollPosition, Val, ZIndex,
        },
    };

    use crate::{GameState, Inventory, ItemId, PresentingItem, QuickSlotUi, SelectionMenu};

    /// Side length of nodes containing images
    const NODE_SIDES: f32 = 64. * 2.;
    /// Gap between items on the scrollable list
    const SCROLL_ITEM_GAP: f32 = 4.;
    /// [`ItemNameBox`] width
    const ITEM_NAME_BOX_WIDTH: f32 = 112. * 2.;
    /// [`ItemNameBox`] height
    const ITEM_NAME_BOX_HEIGHT: f32 = 32. * 2.;

    /// Plugin for the Single list selection menu.
    ///
    /// All items in the inventory are presented in a single horizontal row, and are
    /// selected by using the [`KeyCode::ArrowLeft`] or [`KeyCode::ArrowRight`].
    pub struct SingleSelectionMenuPlugin;

    impl Plugin for SingleSelectionMenuPlugin {
        fn build(&self, app: &mut bevy::app::App) {
            // Creates the UI
            app.add_systems(
                Startup,
                (create_ui, add_cursor_to_first_item)
                    .chain()
                    .after(super::fill_inventory)
                    .after(super::setup_world),
            );

            // Show/hide single selection menu UI
            app.add_systems(
                OnEnter(SingleSelectionMenuState::Shown),
                show_selection_menu,
            )
            .add_systems(OnExit(SingleSelectionMenuState::Shown), hide_selection_menu);

            // Update item name box text on cursor move
            app.add_observer(drop_item_name).add_observer(add_item_name);

            // Moves [`Cursor`]
            app.add_systems(
                Update,
                (move_cursor, tween_cursor).run_if(in_state(SingleSelectionMenuState::Shown)),
            );

            // Adds item to the quick slot when closing selection menu
            app.add_systems(OnExit(SingleSelectionMenuState::Shown), select_item);

            // Single Selection Menu computed state
            app.add_computed_state::<SingleSelectionMenuState>();
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum SingleSelectionMenuState {
        Hidden,
        Shown,
    }

    impl ComputedStates for SingleSelectionMenuState {
        type SourceStates = (GameState, SelectionMenu);

        fn compute(sources: Self::SourceStates) -> Option<Self> {
            match sources {
                (GameState::SelectionMenu, SelectionMenu::Single) => Some(Self::Shown),
                (GameState::Game, SelectionMenu::Single) => Some(Self::Hidden),
                _ => None,
            }
        }
    }

    /// Marker component for the current selected item
    #[derive(Debug, Component)]
    struct Cursor;

    /// Marker component for the current selected item
    #[derive(Debug, Component)]
    struct Tweening {
        /// Index of the node that the [`Cursor`] was on before starting
        start: usize,
        /// Index of the destination node of the [`Cursor`]
        end: usize,
        /// Time passed since the start of the tweening, this is not real time
        time: f32,
    }

    /// Marker component for the ui root
    #[derive(Debug, Component)]
    struct SingleSelectionMenu;

    /// Marker component for the ui node with the list of items
    #[derive(Debug, Component)]
    struct SingleSelectionMenuScroll {
        cursor: usize,
    }

    /// Marker component for items presented on the selection menu
    #[derive(Debug, Component)]
    struct SingleSelectionMenuItem;

    /// Marker component for the box that shows the item name
    #[derive(Debug, Component)]
    struct ItemNameBox;

    /// Shows the UI for the [`SingleSelectionMenu`]
    fn show_selection_menu(
        mut commands: Commands,
        selection_menu: Single<Entity, With<SingleSelectionMenu>>,
    ) {
        commands
            .entity(*selection_menu)
            .insert(Visibility::Inherited);
    }

    /// Hides the UI for the [`SingleSelectionMenu`]
    fn hide_selection_menu(
        mut commands: Commands,
        selection_menu: Single<Entity, With<SingleSelectionMenu>>,
    ) {
        commands.entity(*selection_menu).insert(Visibility::Hidden);
    }

    /// Adds [`Cursor`] to the first [`SingleSelectionMenuItem`]
    fn add_cursor_to_first_item(
        mut commands: Commands,
        items: Query<Entity, With<SingleSelectionMenuItem>>,
    ) {
        let first = items.iter().next().unwrap();
        commands.entity(first).insert(Cursor);
    }

    /// Drops the text from the [`ItemNameBox`]
    fn drop_item_name(
        _trigger: On<Replace, Cursor>,
        mut commands: Commands,
        item_name_box: Single<Entity, With<ItemNameBox>>,
    ) {
        commands.entity(*item_name_box).despawn_children();
    }

    /// Add [`Name`] of the current item pointed to by [`Cursor`]
    /// to the [`ItemNameBox`]
    fn add_item_name(
        trigger: On<Insert, Cursor>,
        mut commands: Commands,
        presenting: Query<&PresentingItem, With<SingleSelectionMenuItem>>,
        names: Query<&Name, With<ItemId>>,
        item_name_box: Single<Entity, With<ItemNameBox>>,
    ) {
        let Ok(presented) = presenting.get(trigger.target()) else {
            unreachable!("Cursor should only ever be added to SingleSelectionMenuItems");
        };
        let Ok(name) = names.get(presented.0) else {
            unreachable!("Cursor should only ever be added to SingleSelectionMenuItems");
        };
        commands.entity(*item_name_box).insert(children![(
            Node {
                width: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_self: AlignSelf::Center,
                ..Default::default()
            },
            children![(
                Text::new(name.to_string()),
                TextFont {
                    font_size: 12.,
                    ..Default::default()
                },
                TextColor(Color::BLACK)
            )]
        )]);
    }

    /// Moves [`Cursor`] in response to [`KeyCode::ArrowLeft`] or [`KeyCode::ArrowRight`] key presses
    fn move_cursor(
        mut commands: Commands,
        key_input: Res<ButtonInput<KeyCode>>,
        tweening: Single<(
            Entity,
            &mut SingleSelectionMenuScroll,
            &Children,
            Has<Tweening>,
        )>,
    ) {
        let (entity, mut scroll, children, tweening) = tweening.into_inner();
        if tweening {
            return;
        }

        let to_left = key_input.pressed(KeyCode::ArrowLeft);
        let to_right = key_input.pressed(KeyCode::ArrowRight);

        let move_to = if to_right {
            Some((scroll.cursor, (scroll.cursor + 1) % children.len()))
        } else if to_left {
            Some((
                scroll.cursor,
                scroll.cursor.checked_sub(1).unwrap_or(children.len() - 1),
            ))
        } else {
            None
        };

        if let Some((prev, next)) = move_to {
            scroll.cursor = next;
            commands.entity(children[prev]).remove::<Cursor>();
            commands.entity(children[next]).insert(Cursor);
            commands.entity(entity).insert(Tweening {
                start: prev,
                end: next,
                time: 0.,
            });
        }
    }

    /// Scrolls the [`SingleSelectMenuScroll`] so that [`Cursor`] is the middle item
    fn tween_cursor(
        mut commands: Commands,
        scroll: Single<
            (Entity, &mut ScrollPosition, &mut Tweening),
            With<SingleSelectionMenuScroll>,
        >,
        time: Res<Time>,
    ) {
        let (entity, mut scroll, mut tweening) = scroll.into_inner();
        let origin = (NODE_SIDES + SCROLL_ITEM_GAP) * tweening.start as f32;
        let destination = (NODE_SIDES + SCROLL_ITEM_GAP) * tweening.end as f32;

        let t = tweening.time + (time.delta_secs() * 4.);
        if t >= 1. {
            scroll.offset_x = destination;
            commands.entity(entity).remove::<Tweening>();
        } else {
            tweening.time = t;
            let final_position = origin + (destination - origin) * t;
            scroll.offset_x = final_position;
        }
    }

    fn select_item(
        mut commands: Commands,
        cursor: Single<&PresentingItem, With<Cursor>>,
        quick_slot: Single<Entity, With<QuickSlotUi>>,
    ) {
        commands.entity(*quick_slot).insert(cursor.clone());
    }

    fn create_ui(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        items: Single<&Children, With<Inventory>>,
    ) {
        commands.spawn((
            Visibility::Hidden,
            SingleSelectionMenu,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..Default::default()
            },
            ZIndex(1),
            children![
                (
                    Node {
                        top: Val::Percent(30.),
                        left: Val::Px(0.),
                        width: Val::Percent(100.),
                        row_gap: Val::Px(32.),
                        flex_direction: FlexDirection::Column,
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    children![
                        (
                            SingleSelectionMenuScroll { cursor: 0 },
                            Node {
                                overflow: Overflow::scroll_x(),
                                width: Val::Px(NODE_SIDES),
                                height: Val::Px(NODE_SIDES),
                                column_gap: Val::Px(SCROLL_ITEM_GAP),
                                flex_direction: FlexDirection::Row,
                                ..Default::default()
                            },
                            Children::spawn(SpawnIter(
                                items
                                    .iter()
                                    .map(|child| (
                                        SingleSelectionMenuItem,
                                        PresentingItem(*child),
                                        Node {
                                            width: Val::Px(NODE_SIDES),
                                            height: Val::Px(NODE_SIDES),
                                            ..Default::default()
                                        }
                                    ))
                                    .collect::<Vec<_>>()
                                    .into_iter()
                            ))
                        ),
                        (
                            ItemNameBox,
                            Node {
                                width: Val::Px(ITEM_NAME_BOX_WIDTH),
                                height: Val::Px(ITEM_NAME_BOX_HEIGHT),
                                flex_direction: FlexDirection::Row,
                                ..Default::default()
                            },
                            ImageNode {
                                image: asset_server
                                    .load("textures/rpg/ui/generic-rpg-ui-text-box.png"),
                                ..Default::default()
                            }
                        )
                    ]
                ),
                (
                    Node {
                        top: Val::Percent(30.),
                        left: Val::Px(0.),
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    children![(
                        Node {
                            width: Val::Px(NODE_SIDES),
                            height: Val::Px(NODE_SIDES),
                            ..Default::default()
                        },
                        ImageNode {
                            image: asset_server
                                .load("textures/fantasy_ui_borders/panel-border-010.png"),
                            ..Default::default()
                        }
                    )]
                )
            ],
        ));
    }
}

/// Creates a world for visual purpose
fn setup_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawns a camera
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedVertical {
        viewport_height: 300.,
    };
    commands.spawn((Camera2d, Projection::Orthographic(projection)));

    // Spawn some characters to be on screen when the selection menu is closed
    let gabe_layout = TextureAtlasLayout::from_grid(UVec2::splat(24), 7, 1, None, None);
    let gabe_texture_atlas_layout = asset_server.add(gabe_layout);
    commands.spawn((
        Transform::from_xyz(-20., 0., 0.),
        Sprite::from_atlas_image(
            asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png"),
            TextureAtlas {
                layout: gabe_texture_atlas_layout,
                index: 0,
            },
        ),
    ));
    commands.spawn((
        Transform::from_xyz(0., 40., 0.),
        Sprite::from_image(asset_server.load("textures/rpg/chars/vendor/generic-rpg-vendor.png")),
    ));

    // Spawns the UI hierarchy for the quick slot
    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::RowReverse,
            ..Default::default()
        },
        ZIndex(0),
        UiBackground,
        BackgroundColor(Color::BLACK.with_alpha(0.)),
        children![
            (
                Node {
                    margin: UiRect::all(Val::Px(8.)),
                    top: Val::Px(0.),
                    right: Val::Px(0.),
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                children![(
                    QuickSlotUi,
                    Node {
                        width: Val::Px(64.),
                        height: Val::Px(64.),
                        ..Default::default()
                    },
                ),]
            ),
            (
                Node {
                    margin: UiRect::all(Val::Px(8.)),
                    top: Val::Px(0.),
                    right: Val::Px(0.),
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                children![(
                    Node {
                        width: Val::Px(64.),
                        height: Val::Px(64.),
                        ..Default::default()
                    },
                    ImageNode {
                        image: asset_server
                            .load("textures/fantasy_ui_borders/panel-border-010.png"),
                        ..Default::default()
                    }
                )]
            )
        ],
    ));
}

fn darker_ui_background(mut commands: Commands, ui_background: Single<Entity, With<UiBackground>>) {
    commands
        .entity(*ui_background)
        .insert(TargetAlpha(DARK_UI_BACKGROUND));
}

fn lighten_ui_background(
    mut commands: Commands,
    ui_background: Single<Entity, With<UiBackground>>,
) {
    commands.entity(*ui_background).insert(TargetAlpha(0.));
}

type UpdateAlphaQuery<'w, 's, T> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut T,
        &'static TargetAlpha,
        Has<FixedAlpha>,
    ),
>;

/// Updates the alpha of a [`BackgroundColor`]
fn update_ui_background(
    mut commands: Commands,
    mut updating_ui_backgrounds: UpdateAlphaQuery<BackgroundColor>,
    time: Res<Time>,
) {
    for (entity, mut background_color, target_alpha, fixed_alpha) in
        updating_ui_backgrounds.iter_mut()
    {
        if fixed_alpha || update_alpha(&mut background_color.0, target_alpha.0, time.delta_secs()) {
            commands.entity(entity).remove::<TargetAlpha>();
        }
    }
}

/// Updates the alpha of a [`ImageNode`]
fn update_image_node_alpha(
    mut commands: Commands,
    mut updating_ui_backgrounds: UpdateAlphaQuery<ImageNode>,
    time: Res<Time>,
) {
    for (entity, mut image_node, target_alpha, fixed_alpha) in updating_ui_backgrounds.iter_mut() {
        if fixed_alpha || update_alpha(&mut image_node.color, target_alpha.0, time.delta_secs()) {
            commands.entity(entity).remove::<TargetAlpha>();
        }
    }
}

/// Update the alpha of an object that implements [`Alpha`] using a exponential decay function.
///
/// Returns a boolean indicating if it has reached close enough to the target
fn update_alpha<T>(mut alpha: T, target_alpha: f32, period: f32) -> bool
where
    T: BorrowMut<Color>,
{
    let alpha = alpha.borrow_mut();
    // Exponential decay function on the difference between current background alpha and target
    let old_alpha = alpha.alpha();
    let decay = (target_alpha - old_alpha) * ops::powf(1. - DECAY_FACTOR, period);
    let new_alpha = target_alpha - decay;
    if (new_alpha - target_alpha).abs() < 1e-2 {
        bevy::log::debug!("Removing TargetUiBackgroundAlpha");
        alpha.set_alpha(target_alpha);
        true
    } else {
        alpha.set_alpha(new_alpha);
        false
    }
}

/// Spawn an inventory and fill it with items
fn fill_inventory(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load_with_settings(
        "textures/food_kenney.png",
        |image_settings: &mut ImageLoaderSettings| {
            image_settings.sampler = ImageSampler::linear();
        },
    );
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 7, 6, None, None);
    let layout_handle = asset_server.add(layout);

    let item_maker =
        |name, image: &Handle<Image>, layout: &Handle<TextureAtlasLayout>, id: u8, category| Item {
            name: Name::new(name),
            sprite: Sprite::from_atlas_image(
                image.clone(),
                TextureAtlas {
                    layout: layout.clone(),
                    index: usize::from(id),
                },
            ),
            item_id: ItemId(id),
            category,
        };

    commands.spawn((
        Inventory,
        Transform::default(),
        Visibility::Hidden,
        Children::spawn(SpawnIter(
            vec![
                item_maker(
                    "Half Avocado",
                    &image,
                    &layout_handle,
                    0,
                    ItemCategory::Fruit,
                ),
                item_maker("Half Apple", &image, &layout_handle, 1, ItemCategory::Fruit),
                item_maker("Apple", &image, &layout_handle, 2, ItemCategory::Fruit),
                item_maker("Avocado", &image, &layout_handle, 3, ItemCategory::Fruit),
                item_maker("Bacon", &image, &layout_handle, 4, ItemCategory::Ingredient),
                item_maker(
                    "Fried Bacon",
                    &image,
                    &layout_handle,
                    5,
                    ItemCategory::Protein,
                ),
                item_maker("Flour", &image, &layout_handle, 6, ItemCategory::Ingredient),
                // Maybe??
                item_maker("Sugar", &image, &layout_handle, 7, ItemCategory::Ingredient),
                item_maker("Banana", &image, &layout_handle, 8, ItemCategory::Fruit),
                item_maker("Wine", &image, &layout_handle, 9, ItemCategory::Liquid),
                item_maker(
                    "Turnip",
                    &image,
                    &layout_handle,
                    10,
                    ItemCategory::Vegetable,
                ),
                item_maker(
                    "Ketchup",
                    &image,
                    &layout_handle,
                    11,
                    ItemCategory::Condiment,
                ),
                item_maker(
                    "Mustard",
                    &image,
                    &layout_handle,
                    12,
                    ItemCategory::Condiment,
                ),
                item_maker(
                    "Olive Oil",
                    &image,
                    &layout_handle,
                    13,
                    ItemCategory::Ingredient,
                ),
                // Don't @ me
                item_maker("Lamen", &image, &layout_handle, 14, ItemCategory::Soup),
                item_maker("Soup", &image, &layout_handle, 15, ItemCategory::Soup),
                item_maker(
                    "Chicken Soup",
                    &image,
                    &layout_handle,
                    16,
                    ItemCategory::Soup,
                ),
                item_maker("Bowl", &image, &layout_handle, 17, ItemCategory::Tool),
                item_maker(
                    "Sliced Bread",
                    &image,
                    &layout_handle,
                    18,
                    ItemCategory::Ingredient,
                ),
                item_maker(
                    "Broccoli",
                    &image,
                    &layout_handle,
                    19,
                    ItemCategory::Vegetable,
                ),
                item_maker(
                    "Double Cheeseburger",
                    &image,
                    &layout_handle,
                    20,
                    ItemCategory::Hamburger,
                ),
                item_maker(
                    "Cheeseburger",
                    &image,
                    &layout_handle,
                    21,
                    ItemCategory::Hamburger,
                ),
                item_maker(
                    "Double Deluxe Burger",
                    &image,
                    &layout_handle,
                    22,
                    ItemCategory::Hamburger,
                ),
                item_maker(
                    "Deluxe Burger",
                    &image,
                    &layout_handle,
                    23,
                    ItemCategory::Hamburger,
                ),
                item_maker(
                    "Cabagge",
                    &image,
                    &layout_handle,
                    24,
                    ItemCategory::Vegetable,
                ),
                item_maker(
                    "Birthday Cake",
                    &image,
                    &layout_handle,
                    25,
                    ItemCategory::Cake,
                ),
                item_maker(
                    "Cake Spatula",
                    &image,
                    &layout_handle,
                    26,
                    ItemCategory::Tool,
                ),
                item_maker(
                    "Strawberry Cake",
                    &image,
                    &layout_handle,
                    27,
                    ItemCategory::Cake,
                ),
                item_maker(
                    "Canned Beans",
                    &image,
                    &layout_handle,
                    28,
                    ItemCategory::Canned,
                ),
                item_maker(
                    "Canned Fish",
                    &image,
                    &layout_handle,
                    29,
                    ItemCategory::Canned,
                ),
                item_maker(
                    "Canned Soup",
                    &image,
                    &layout_handle,
                    30,
                    ItemCategory::Canned,
                ),
                item_maker(
                    "Chocolate 1",
                    &image,
                    &layout_handle,
                    31,
                    ItemCategory::Chocolate,
                ),
                item_maker(
                    "Chocolate 2",
                    &image,
                    &layout_handle,
                    32,
                    ItemCategory::Chocolate,
                ),
                item_maker(
                    "Carrot",
                    &image,
                    &layout_handle,
                    33,
                    ItemCategory::Vegetable,
                ),
                item_maker("Soy Milk", &image, &layout_handle, 34, ItemCategory::Liquid),
                item_maker("Milk", &image, &layout_handle, 35, ItemCategory::Liquid),
                item_maker(
                    "Cauliflower",
                    &image,
                    &layout_handle,
                    36,
                    ItemCategory::Vegetable,
                ),
                item_maker(
                    "Celery",
                    &image,
                    &layout_handle,
                    37,
                    ItemCategory::Vegetable,
                ),
                item_maker(
                    "Cheese Slice",
                    &image,
                    &layout_handle,
                    38,
                    ItemCategory::Cheese,
                ),
                item_maker(
                    "Cheese Spatula",
                    &image,
                    &layout_handle,
                    39,
                    ItemCategory::Tool,
                ),
                item_maker(
                    "Cheese Wheel",
                    &image,
                    &layout_handle,
                    40,
                    ItemCategory::Cheese,
                ),
                item_maker("Cherry", &image, &layout_handle, 41, ItemCategory::Fruit),
            ]
            .into_iter(),
        )),
    ));
}
