//! Builds a dockable layout from a `DockTree` and edits the tree at runtime.
//!
//! Drag the splitters to resize groups, click tabs to switch between them and drag tabs to
//! reorder them or move them to another group. The keyboard shortcuts listed at the top of the
//! window edit the tree, and the UI follows.
//!
//! `build_panels` fills the content entity the dock spawns for each tab.
//! Clicking a panel's counter updates its `DockPanelTitle`.

use bevy::{
    picking::{
        cursor::{CursorIconPlugin, EntityCursor},
        events::PointerClick,
    },
    prelude::*,
    ui::Selected,
    ui_widgets::{
        dock_self_update, ControlOrientation, DockChange, DockEdge, DockEdit, DockNodeId,
        DockPanelContent, DockPanelTitle, DockSystems, DockTabBinding, DockTree, SplitPane,
        SplitPaneHandle, TabDragging, TabInsertionPreview, TabList,
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
struct Counter {
    /// The number of times the panel has been clicked.
    clicks: u32,
    /// The content entity
    content: Entity,
    /// The title to show in the tab, with the count appended.
    title: String,
}

#[derive(Resource, Default)]
struct LastGroup(Option<DockNodeId>);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CursorIconPlugin))
        .init_resource::<LastGroup>()
        .add_systems(Startup, setup)
        // Systems that build paneles belong in this set;
        // it runs before layout, so you can avoid a frame of empty panels.
        .add_systems(PostUpdate, build_panels.in_set(DockSystems::Panels))
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

fn setup(mut commands: Commands) {
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

fn build_panels(
    panels: Query<(Entity, &DockPanelContent), Added<DockPanelContent>>,
    mut notes: Local<u32>,
    mut commands: Commands,
) {
    for (content, panel) in &panels {
        let (title, color, counter) = match panel.panel.as_str() {
            "outliner" => ("Outliner".into(), Color::srgb(0.18, 0.22, 0.30), true),
            "layers" => ("Layers".into(), Color::srgb(0.24, 0.18, 0.28), true),
            "assets" => ("Assets".into(), Color::srgb(0.16, 0.24, 0.20), false),
            "viewport" => ("Viewport".into(), Color::srgb(0.08, 0.09, 0.11), true),
            "inspector" => ("Inspector".into(), Color::srgb(0.28, 0.22, 0.16), false),
            "preview" => ("Preview".into(), Color::srgb(0.14, 0.20, 0.26), true),
            "note" => {
                *notes += 1;
                (
                    format!("Note {}", *notes),
                    Color::srgb(0.26, 0.26, 0.18),
                    true,
                )
            }
            _ => continue,
        };
        build_content(&mut commands, content, title, color, counter);
    }
}

fn build_content(
    commands: &mut Commands,
    content: Entity,
    title: String,
    color: Color,
    counter: bool,
) {
    commands
        .entity(content)
        .insert((DockPanelTitle::new(title.clone()), BackgroundColor(color)))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    padding: UiRect::all(px(16)),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((Text::new(title.clone()), TextFont::from_font_size(24.0)));
                    if counter {
                        parent
                            .spawn((
                                Counter {
                                    clicks: 0,
                                    content,
                                    title: title.clone(),
                                },
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

fn count_click(
    mut click: On<PointerClick>,
    mut counters: Query<(&mut Counter, &mut Text)>,
    mut commands: Commands,
) {
    click.propagate(false);
    if let Ok((mut counter, mut text)) = counters.get_mut(click.entity) {
        counter.clicks += 1;
        text.0 = format!("Clicked {} times", counter.clicks);
        let title = format!("{} ({})", counter.title, counter.clicks);
        commands
            .entity(counter.content)
            .insert(DockPanelTitle::new(title));
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
