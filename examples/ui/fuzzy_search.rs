//! This example demonstrates how to use the `bevy_fuzzy` crate to implement a fuzzy search feature.

use bevy::{
    feathers::{
        containers::*,
        controls::{
            FeathersListRow, FeathersListView, FeathersTextInput, FeathersTextInputContainer,
        },
        dark_theme::create_dark_theme,
        display::{caption, label_dim},
        theme::UiTheme,
        FeathersPlugins,
    },
    fuzzy::FuzzyPattern,
    input_focus::{tab_navigation::TabIndex, AutoFocus},
    prelude::*,
    text::{EditableText, Placeholder, TextEditChange},
};

const ITEMS: &[&str] = &[
    "Iron Sword",
    "Steel Sword",
    "Silver Sword",
    "Wooden Shield",
    "Iron Shield",
    "Steel Shield",
    "Health Potion",
    "Greater Health Potion",
    "Mana Potion",
    "Greater Mana Potion",
    "Leather Armor",
    "Iron Armor",
    "Steel Armor",
    "Magic Staff",
    "Fire Staff",
    "Ice Staff",
    "Long Bow",
    "Short Bow",
    "Iron Helmet",
    "Steel Helmet",
];

/// Cache the last query because `TextEditChange` also fires for cursor movements.
#[derive(Component, Default, Clone)]
struct InventorySearch(String);

#[derive(Component, Default, Clone)]
struct SearchResults;

#[derive(Component, Default, Clone)]
struct SearchStatus;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        @root()
    }
}

fn root() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [
            @pane()
            Node {
                width: px(420),
                max_width: percent(100),
            }
            Children [
                @pane_header() Children [
                    @caption("Inventory Search")
                ]
                --
                @pane_body() Children [
                    @FeathersTextInputContainer
                    Node {
                        flex_grow: 0.0,
                        flex_shrink: 0.0,
                    }
                    Children [
                        @FeathersTextInput
                        Placeholder::new("Search items...")
                        InventorySearch::default()
                        TabIndex(0)
                        AutoFocus
                        on(search_inventory)
                    ]
                    --
                    @label_dim(format!("{} / {} items", ITEMS.len(), ITEMS.len()))
                    SearchStatus
                    --
                    @item_list(ITEMS)
                    SearchResults
                ]
            ]
        ]
    }
}

fn item_rows(items: &[&str]) -> impl SceneList {
    items
        .iter()
        .map(|item| {
            bsn! {
                @FeathersListRow() Children [
                    @caption(item.to_owned())
                ]
            }
        })
        .collect::<Vec<_>>()
}

fn item_list(items: &[&str]) -> impl Scene {
    bsn! {
        @subpane() Children [
            @subpane_header() Children [
                @caption("Item List")
            ]
            --
            @subpane_body() Children [
                @FeathersListView {
                    @rows: { Box::new(item_rows(items)) as Box<dyn SceneList> }
                }
                Node {
                    height: px(240),
                }
            ]
        ]
    }
}

fn search_inventory(
    change: On<TextEditChange>,
    mut inputs: Query<(&EditableText, &mut InventorySearch)>,
    results: Single<Entity, With<SearchResults>>,
    mut status: Single<&mut Text, With<SearchStatus>>,
    mut commands: Commands,
) {
    let Ok((input, mut search)) = inputs.get_mut(change.event_target()) else {
        return;
    };
    let query = input.value();
    if search.0 == query {
        return;
    }
    search.0 = query.into_owned();

    // `rank` filters and sorts by match quality. An empty query restores inventory order.
    let items: Vec<_> = FuzzyPattern::new(&search.0)
        .rank(ITEMS, |item| *item)
        .into_iter()
        .map(|matched| ITEMS[matched.index])
        .collect();

    status.0 = if items.is_empty() {
        "0 matches - No matching items".to_owned()
    } else {
        format!("{} / {} items", items.len(), ITEMS.len())
    };

    // Keep the input and its focus, but reset the result list and its scroll position.
    commands
        .entity(*results)
        .despawn_related::<Children>()
        .apply_scene(item_list(&items));
}