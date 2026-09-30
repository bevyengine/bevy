---
title: Feathers tabs
authors: ["@jbuehler23"]
pull_requests: [25987]
---

`bevy_feathers` now has styled tabs built on the headless `TabList` and `Tab` widgets. `FeathersTabList` is the strip and `FeathersTab` is a tab header whose caption can hold text, icons or your own controls. Tabs are themed for hover, selection, focus, dragging and disabled states.

When dragging is enabled, a copy of the dragged tab header follows the pointer and a thin accent line marks where it will land, including on another list during an external drag. The copy is shown above the nearest `DragOverlayRoot`, so add that component to your root UI node.

As with the headless widgets, selection and moves are proposed with events, so your app decides what happens.

```rust
bsn! {
    @FeathersTabList {
        @drag: TabDragMode::Reorder,
        @selected: OptionTemplate::Some(#home),
    }
    on(tablist_self_update)
    on(apply_tab_move)
    Children [
        #home
        @FeathersTab { @caption: bsn! { @caption("Home") } }
        TabLocked
        --
        @FeathersTab { @caption: bsn! { @caption("Scene") } }
    ]
}

fn apply_tab_move(moved: On<TabMoved>, mut commands: Commands) {
    commands
        .entity(moved.to_strip)
        .insert_child(moved.index, moved.tab);
}
```

See the `feathers_gallery` example for reordering and dragging tabs between lists.
