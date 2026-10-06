---
title: Dock UI
authors: ["@jbuehler23"]
pull_requests: []
---

`DockPlugin` builds UI for every entity with a `DockTree`. It is part of `UiWidgetsPlugins`, so `DefaultPlugins` includes it. Each split becomes a split pane with draggable handles, and each tab group becomes a tab list above a content area. When the tree changes, the UI is updated in place: groups and tabs that are still present keep their entities.

Panel content is registered per `DockPanelKey` in the `DockPanels` resource, with a title for its tabs. Content is built once per tab and kept until the tab is closed, so its state survives switching tabs and moving them between groups. Inactive tabs are hidden with `Display::None`.

```rust
fn setup(mut commands: Commands, mut panels: ResMut<DockPanels>) {
    panels.register("viewport", "Viewport", |content| {
        content.with_child(Text::new("Viewport"));
    });

    let mut tree = DockTree::new();
    let center = tree.root();
    tree.add_tab(center, "viewport").unwrap();
    tree.split(center, DockEdge::Left, "outliner").unwrap();

    commands
        .spawn((tree, Node::default()))
        .observe(dock_self_update);
}
```

Like other headless widgets, the dock is controlled. Clicking a tab or dragging a splitter sends a `DockChange` to the dock entity, and `dock_self_update` applies it to the tree. Apps that manage the layout themselves can observe `DockChange` instead. The UI has layout only and no colors, so apps can style it as they like.
