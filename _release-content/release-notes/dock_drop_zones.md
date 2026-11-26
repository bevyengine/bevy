---
title: Dock drop zones
authors: ["@jbuehler23"]
pull_requests: [26527]
---

Tabs in a dock can now be dropped on the content of a group as well as on its tab list. Dropping near an edge of a group moves the tab into a new group on that side, splitting the space, and dropping in the middle moves it into the group.

While a tab is dragged, `DockPlugin` adds a `DockDropArea` to each group's content area, after the active tab's content. The area under the pointer gets a `DockDropTarget` naming its `DockDropZone`, which styling can use to preview where the tab will land. A drop becomes a `DockChange` with `DockEdit::MoveToEdge` or `DockEdit::MoveTab`, so the dock stays controlled. A drop on a tab list still reorders or moves the tab as before.
