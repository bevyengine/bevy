//! Builds a dockable layout from a `DockTree` and edits the tree at runtime.
//!
//! Drag the splitters to resize groups, click tabs to switch between them and drag tabs to
//! reorder them or move them to another group. The keyboard shortcuts listed at the top of the
//! window edit the tree, and the UI follows.

use bevy::{
    picking::{
        cursor::{CursorIconPlugin, EntityCursor},
        events::PointerClick,
    },
    prelude::*,
    ui::Selected,
    ui_widgets::{
        dock_self_update, ControlOrientation, DockChange, DockEdge, DockEdit, DockNodeId,
        DockPanels, DockTabBinding, DockTree, SplitPane, SplitPaneHandle, TabDragging,
        TabInsertionPreview, TabList,
    },
    window::SystemCursorIcon,
};

const TAB: Color = Color::srgb(0.16, 0.17, 0.20);
const TAB_SELECTED: Color = Color::srgb(0.30, 0.33, 0.40);
const TAB_BAR: Color = Color::srgb(0.10, 0.10, 0.12);
const HANDLE: Color = Color::srgb(0.05, 0.05, 0.06);
const TAB_DRAGGED: Color = Color::srgb(0.45, 0.50, 0.62);
const INSERTION: Color = Color::srgb(0.95, 0.80, 0.30);

const HELP: &str =
    "N: add a note to the last clicked group    S: split the viewport to the right    \
W: close the active tab of the first group    R: reset";

#[derive(Component)]
struct Dock;

#[derive(Component)]
struct Counter(u32);

#[derive(Resource, Default)]
struct LastGroup(Option<DockNodeId>);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CursorIconPlugin))
        .init_resource::<LastGroup>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (edit_tree, style_tabs, style_tab_lists, style_handles),
        )
        .run();
}

fn initial_tree() -> DockTree {
    let mut tree = DockTree::new();
    let center = tree.root();
    tree.add_tab(center, "viewport").unwrap();
    let (left, outliner) = tree.split(center, DockEdge::Left, "outliner").unwrap();
    tree.add_tab(left, "layers").unwrap();
    tree.set_active(left, outliner).unwrap();
    tree.split(left, DockEdge::Bottom, "assets").unwrap();
    tree.split(center, DockEdge::Right, "inspector").unwrap();
    tree.set_sizes(tree.root(), &[1.0, 2.5, 1.0]).unwrap();
    tree
}

fn setup(mut commands: Commands, mut panels: ResMut<DockPanels>) {
    panels
        .register("outliner", "Outliner", |content| {
            panel(content, Color::srgb(0.18, 0.22, 0.30), "Outliner", true);
        })
        .register("layers", "Layers", |content| {
            panel(content, Color::srgb(0.24, 0.18, 0.28), "Layers", true);
        })
        .register("assets", "Assets", |content| {
            panel(content, Color::srgb(0.16, 0.24, 0.20), "Assets", false);
        })
        .register("viewport", "Viewport", |content| {
            panel(content, Color::srgb(0.08, 0.09, 0.11), "Viewport", true);
        })
        .register("inspector", "Inspector", |content| {
            panel(content, Color::srgb(0.28, 0.22, 0.16), "Inspector", false);
        })
        .register("preview", "Preview", |content| {
            panel(content, Color::srgb(0.14, 0.20, 0.26), "Preview", true);
        })
        .register("note", "Note", |content| {
            panel(content, Color::srgb(0.26, 0.26, 0.18), "Note", true);
        });

    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new(HELP),
                TextFont::from_font_size(14.0),
                Node {
                    padding: UiRect::all(px(8)),
                    ..default()
                },
                BackgroundColor(TAB_BAR),
            ));
            parent
                .spawn((
                    Dock,
                    initial_tree(),
                    Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        min_height: px(0),
                        ..default()
                    },
                ))
                .observe(dock_self_update)
                .observe(remember_group);
        });
}

fn panel(content: &mut EntityCommands, color: Color, title: &str, counter: bool) {
    content
        .insert(BackgroundColor(color))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    padding: UiRect::all(px(16)),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((Text::new(title), TextFont::from_font_size(24.0)));
                    if counter {
                        parent
                            .spawn((
                                Counter(0),
                                Text::new("Clicked 0 times"),
                                Node {
                                    padding: UiRect::axes(px(12), px(6)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                            ))
                            .observe(count_click);
                    }
                });
        });
}

fn count_click(mut click: On<PointerClick>, mut counters: Query<(&mut Counter, &mut Text)>) {
    click.propagate(false);
    if let Ok((mut counter, mut text)) = counters.get_mut(click.entity) {
        counter.0 += 1;
        text.0 = format!("Clicked {} times", counter.0);
    }
}

fn remember_group(change: On<DockChange>, mut last: ResMut<LastGroup>) {
    if let DockEdit::Activate { leaf, .. } = change.edit {
        last.0 = Some(leaf);
    }
}

fn edit_tree(
    keys: Res<ButtonInput<KeyCode>>,
    mut docks: Query<&mut DockTree, With<Dock>>,
    mut last: ResMut<LastGroup>,
) {
    let Ok(mut tree) = docks.single_mut() else {
        return;
    };
    let first = tree.leaves().next().map(|(id, _)| id);
    if keys.just_pressed(KeyCode::KeyN) {
        let group = last.0.filter(|id| tree.leaf(*id).is_some()).or(first);
        if let Some(group) = group {
            tree.add_tab(group, "note").unwrap();
        }
    }
    if keys.just_pressed(KeyCode::KeyS) {
        let target = tree
            .find_panel("viewport")
            .map_or(tree.root(), |(leaf, _)| leaf);
        tree.split(target, DockEdge::Right, "preview").unwrap();
    }
    if keys.just_pressed(KeyCode::KeyW) {
        let active = first.and_then(|id| tree.leaf(id)?.active());
        if let Some(tab) = active {
            tree.remove_tab(tab).unwrap();
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        *tree = initial_tree();
        last.0 = None;
    }
}

fn style_tabs(
    mut commands: Commands,
    mut tabs: Query<
        (
            Entity,
            Has<Selected>,
            Has<TabDragging>,
            &mut Node,
            Option<&BackgroundColor>,
            Option<&BorderColor>,
        ),
        With<DockTabBinding>,
    >,
    previews: Query<(&Children, &TabInsertionPreview)>,
) {
    let mut marks: Vec<(Entity, bool)> = Vec::new();
    for (children, preview) in &previews {
        for entry in &preview.entries {
            match children.get(entry.slot) {
                Some(&tab) => marks.push((tab, true)),
                None => marks.extend(children.last().map(|&tab| (tab, false))),
            }
        }
    }
    for (tab, selected, dragged, mut node, background, border) in &mut tabs {
        let color = BackgroundColor(match (dragged, selected) {
            (true, _) => TAB_DRAGGED,
            (false, true) => TAB_SELECTED,
            (false, false) => TAB,
        });
        if background != Some(&color) {
            commands.entity(tab).insert(color);
        }
        let mut edges = BorderColor::all(Color::NONE);
        for &(marked, before) in &marks {
            if marked == tab {
                if before {
                    edges.left = INSERTION;
                } else {
                    edges.right = INSERTION;
                }
            }
        }
        if border != Some(&edges) {
            commands.entity(tab).insert(edges);
        }
        if node.border.left != px(2) {
            node.border = UiRect::horizontal(px(2));
        }
    }
}

fn style_tab_lists(mut commands: Commands, lists: Query<Entity, Added<TabList>>) {
    for list in &lists {
        commands.entity(list).insert(BackgroundColor(TAB_BAR));
    }
}

fn style_handles(
    mut commands: Commands,
    handles: Query<(Entity, &ChildOf), Added<SplitPaneHandle>>,
    splits: Query<&SplitPane>,
) {
    for (handle, parent) in &handles {
        let icon = match splits.get(parent.parent()).map(|split| split.orientation) {
            Ok(ControlOrientation::Vertical) => SystemCursorIcon::RowResize,
            _ => SystemCursorIcon::ColResize,
        };
        commands
            .entity(handle)
            .insert((BackgroundColor(HANDLE), EntityCursor::System(icon)));
    }
}
