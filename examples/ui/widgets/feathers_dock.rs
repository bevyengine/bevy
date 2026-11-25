//! Shows a dock styled with feathers.
//!
//! Drag the splitters to resize groups, click tabs to switch between them and drag tabs to
//! reorder them, move them to another group or drop them at the edge of a group to split it.

use bevy::{
    feathers::{
        controls::FeathersDock,
        dark_theme::create_dark_theme,
        display::label_dim,
        theme::{ThemeBackgroundColor, UiTheme},
        tokens, FeathersPlugins,
    },
    prelude::*,
    ui_widgets::{
        dock_self_update, DockEdge, DockPanelContent, DockPanelTitle, DockSystems, DockTree,
        DragOverlayRoot,
    },
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, setup)
        .add_systems(PostUpdate, build_panels.in_set(DockSystems::Panels))
        .run();
}

fn initial_tree() -> DockTree {
    let mut tree = DockTree::new();
    let center = tree.root();
    tree.add_tab(center, "viewport").unwrap();
    let (left, _) = tree.split(center, DockEdge::Left, "outliner").unwrap();
    tree.add_tab(left, "layers").unwrap();
    tree.split(left, DockEdge::Bottom, "assets").unwrap();
    let (right, _) = tree.split(center, DockEdge::Right, "inspector").unwrap();
    tree.add_tab(right, "settings").unwrap();
    tree.set_sizes(tree.root(), &[1.0, 2.5, 1.0]).unwrap();
    tree
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(8)),
                ..default()
            },
            DragOverlayRoot,
            ThemeBackgroundColor(tokens::WINDOW_BG),
        ))
        .with_children(|root| {
            root.spawn((
                FeathersDock,
                initial_tree(),
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    min_height: px(0),
                    ..default()
                },
            ))
            .observe(dock_self_update);
        });
}

fn build_panels(
    panels: Query<(Entity, &DockPanelContent), Added<DockPanelContent>>,
    mut commands: Commands,
) {
    for (content, panel) in &panels {
        let title = match panel.panel.as_str() {
            "viewport" => "Viewport",
            "outliner" => "Outliner",
            "layers" => "Layers",
            "assets" => "Assets",
            "inspector" => "Inspector",
            "settings" => "Settings",
            _ => continue,
        };
        commands
            .entity(content)
            .insert(DockPanelTitle::new(title))
            .queue_spawn_related_scenes::<Children>(bsn_list! {
                Node {
                    padding: UiRect::all(px(8)),
                }
                Children [ @label_dim(format!("{title} panel")) ]
            });
    }
}
