---
title: Feathers tree view widget
authors: ["@jbuehler23"]
pull_requests: [25862]
---

`bevy_feathers` now has a styled tree view built on the headless `TreeView` widget. `FeathersTreeView` is the root container and `FeathersTreeItem` is a row, which draws a rotating chevron for expandable rows, indents itself from `TreeItem::level`, and picks up the same hover, selection, focus and disabled styling as a Feathers list row.

Rows take a `label` scene and an optional list of child rows, and the row's child container is hidden while the row is collapsed, so lazily populated branches work the same way they do in the headless widget. See the `feathers_gallery` example for a tree with a lazily populated branch.
