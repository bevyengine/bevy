//! Demonstrates the behavior-only tree widget in `bevy_ui_widgets`.

use bevy::{
    ecs::template::{EntityTemplate, OptionTemplate},
    input_focus::{
        tab_navigation::{TabGroup, TabNavigationPlugin},
        InputFocus, InputFocusVisible,
    },
    picking::hover::Hovered,
    prelude::*,
    ui::Selected,
    ui_widgets::{
        tree_view_expand_self_update, tree_view_self_update, SelectedTreeItem, TreeItem,
        TreeItemChildren, TreeItemExpandChange, TreeItemToggle, TreeView,
    },
};

#[derive(Component, Default, Clone)]
struct ShowcaseRow;

#[derive(Component, Default, Clone)]
struct ShowcaseHeader;

#[derive(Component, Default, Clone)]
struct ShowcaseToggle;

#[derive(Component, Default, Clone)]
struct LazyBranch;

#[derive(Component, Default, Clone)]
struct Populated;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TabNavigationPlugin))
        .add_systems(Startup, showcase.spawn())
        .add_systems(Update, update_row_styles)
        .run();
}

fn showcase() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(24)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            row_gap: px(12),
        }
        BackgroundColor(Color::srgb(0.06, 0.07, 0.09))
        TabGroup
        Children [
            Text("Entity hierarchy")
            TextFont {
                font_size: FontSize::Px(17.0)
            }
            TextColor(Color::srgb(0.72, 0.76, 0.84))
            --
            TreeView
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                min_width: px(280),
                padding: UiRect::all(px(4)),
            }
            BackgroundColor(Color::srgb(0.10, 0.11, 0.14))
            on(tree_view_self_update)
            on(tree_view_expand_self_update)
            on(populate_lazy_branch)
            @selected_item(#camera_row)
            Children [
                #camera_row
                @row()
                Children [
                    @row_header("Camera", false)
                ]
                --
                @row()
                TreeItem {
                    expanded: true,
                    has_children: true,
                }
                Children [
                    @row_header("Scene", true)
                    --
                    TreeItemChildren
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                    }
                    Children [
                        @row()
                        Children [
                            @row_header("Ground", false)
                        ]
                        --
                        @row()
                        Children [
                            @row_header("Player", false)
                        ]
                    ]
                ]
                --
                LazyBranch
                @row()
                TreeItem {
                    has_children: true,
                }
                Children [
                    @row_header("Assets", true)
                ]
            ]
        ]
    }
}

fn row() -> impl Scene {
    bsn! {
        ShowcaseRow
        TreeItem
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
        }
    }
}

fn row_header(label: &'static str, expandable: bool) -> impl Scene {
    let toggle = if expandable { "+" } else { " " };
    bsn! {
        ShowcaseHeader
        Hovered
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(6),
            padding: UiRect::axes(px(8), px(4)),
            border: UiRect::all(px(1)),
        }
        BackgroundColor(Color::srgb(0.10, 0.11, 0.14))
        BorderColor::all(Color::srgb(0.10, 0.11, 0.14))
        Children [
            ShowcaseToggle
            TreeItemToggle
            Node {
                width: px(12),
            }
            Text({toggle})
            TextFont {
                font_size: FontSize::Px(15.0)
            }
            TextColor(Color::srgb(0.62, 0.66, 0.74))
            --
            Text(label)
            TextFont {
                font_size: FontSize::Px(15.0)
            }
            TextColor(Color::srgb(0.88, 0.89, 0.92))
        ]
    }
}

fn selected_item(item: EntityTemplate) -> impl Scene {
    let item = OptionTemplate::Some(item);
    bsn! {
        SelectedTreeItem({item})
    }
}

/// Spawns the child rows of the "Assets" row the first time it is expanded.
fn populate_lazy_branch(
    change: On<TreeItemExpandChange>,
    lazy: Query<(), (With<LazyBranch>, Without<Populated>)>,
    mut commands: Commands,
) {
    if !change.expanded || !lazy.contains(change.item) {
        return;
    }
    commands.entity(change.item).insert(Populated);
    commands
        .spawn_scene(bsn! {
            TreeItemChildren
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
            }
            Children [
                @row()
                Children [
                    @row_header("Mesh", false)
                ]
                --
                @row()
                Children [
                    @row_header("Material", false)
                ]
            ]
        })
        .insert(ChildOf(change.item));
}

fn update_row_styles(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    rows: Query<(Has<Selected>, &TreeItem), With<ShowcaseRow>>,
    parents: Query<&ChildOf>,
    mut headers: Query<
        (
            &ChildOf,
            &Hovered,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Node,
        ),
        With<ShowcaseHeader>,
    >,
    mut toggles: Query<(Entity, &mut Text), With<ShowcaseToggle>>,
) {
    for (child_of, hovered, mut background, mut border, mut node) in &mut headers {
        let row = child_of.parent();
        let Ok((selected, item)) = rows.get(row) else {
            continue;
        };
        background.0 = match (selected, hovered.get()) {
            (true, _) => Color::srgb(0.18, 0.34, 0.52),
            (false, true) => Color::srgb(0.16, 0.17, 0.21),
            _ => Color::srgb(0.10, 0.11, 0.14),
        };
        border.set_all(if focus_visible.0 && focus.get() == Some(row) {
            Color::srgb(0.45, 0.78, 1.0)
        } else {
            Color::srgb(0.10, 0.11, 0.14)
        });
        node.padding.left = px(8.0 + 14.0 * item.level as f32);
    }

    for (toggle, mut text) in &mut toggles {
        let Some(row) = parents
            .iter_ancestors(toggle)
            .find(|ancestor| rows.contains(*ancestor))
        else {
            continue;
        };
        let Ok((_, item)) = rows.get(row) else {
            continue;
        };
        let glyph = match (item.has_children, item.expanded) {
            (false, _) => " ",
            (true, false) => "+",
            (true, true) => "-",
        };
        if text.0 != glyph {
            text.0 = glyph.to_string();
        }
    }
}
