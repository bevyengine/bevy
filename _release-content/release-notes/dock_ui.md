---
title: Dockable panels
authors: ["@jbuehler23", "@alice-i-cecile"]
pull_requests: [26183]
---

Making complex creations commonly means juggling multiple distinct tasks at once, and games are no different.
In a GUI, the standard paradigm these days is to assign each of these tasks their own tab,
and then allow you to drag and drop them between different regions at will: "docking" them in place.

`bevy_ui_widgets` now ships dockable panels, modelled as a group of nested entities:
- at the root we have the dock itself: an entity with a `DockTree` component and a `Node`, where every node in the tree becomes one of its descendants
- the space owned by the `DockTree` is partitioned by `SplitPane` entities, whose `Pane` children are separated by draggable `SplitPaneHandle` entities
- each `Pane` can hold either a tab group or another `SplitPane`, allowing for recursive splitting
- each tab group is a leaf node with exactly three children: a `TabList` that stores the tab headers as children, a content area where the thing you want to show on the panel goes, and a hidden node, which holds the content of inactive tabs
- each tab is made up of two entities that live under its respective tab group: the `Tab` header, which stays in the list, and the `DockPanelContent` holding the panel's UI, which moves between the content area and the hidden node

Only the active tab's content sits in the content area; every other tab's content is parked in the hidden node, a `Node` with `Display::None`, so switching tabs moves entities rather than rebuilding them.

Setting the content for each tab requires a bit of indirection.
The obvious way would be to spawn the panel straight into its tab as a child entity.
Unfortunately, our requirements complicate that: we want to be able to close down our editor, and reopen it with the tabs, their layout and the values that they set completely intact.

As a result, we've opted for a key-based design for this particular widget, rather than relying solely on entity relations,
regenerating the panel content from scratch based on their `DockPanelKey` whenever the app sees a new `DockPanelContent` component.
By assigning a stable, serializable key to each panel (in combination with a unique `DockTabId`), it becomes much easier to persist this state.

```rust
fn setup_dock_tree(mut commands: Commands) {
    let mut tree = DockTree::new();
    let center = tree.root();
    tree.add_tab(center, "viewport").unwrap();
    tree.split(center, DockEdge::Left, "outliner").unwrap();

    commands.spawn((tree, Node::default())).observe(dock_self_update);
}

fn build_panels(
    panels: Query<(Entity, &DockPanelContent), Added<DockPanelContent>>,
    mut commands: Commands,
) {
    for (content, panel) in &panels {
        let (title, color) = match panel.panel.as_str() {
            "viewport" => ("Viewport", Color::srgb(0.08, 0.09, 0.11)),
            "outliner" => ("Outliner", Color::srgb(0.18, 0.22, 0.30)),
            // Other plugins can decide how to handle their own panels independently
            _ => continue,
        };
        commands
            .entity(content)
            // We can set the title of each panel directly
            // but the DockPanelKey is used as a fallback if none is given.
            .insert((DockPanelTitle::new(title), BackgroundColor(color)));
    }
}

app.add_systems(Startup, setup_dock_tree)
    .add_systems(PostUpdate, build_panels.in_set(DockSystems::Panels))
```

Tabs can be dragged to reorder them within a group or to move them to another group of the same dock. The tab lists use `TabDragMode::External`, and a drop becomes a `DockEdit::MoveTab` applied with `DockTree::move_tab`, so a group emptied by the move is removed from the layout.

Like other headless widgets, the dock is a controlled widget. Clicking a tab, dragging a tab or dragging a splitter sends a `DockChange` to the dock entity, and `dock_self_update` applies it to the tree. Apps that manage the layout themselves can observe `DockChange` instead. The UI has layout only and no colors, so apps can style it as they like.
