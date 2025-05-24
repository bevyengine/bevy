---
title: Headless Widgets
authors: ["@viridia"]
pull_requests: [19238, 19349]
---

Bevy's `Button` and `Interaction` components have been around for a long time. Unfortunately
these components have a number of shortcomings, such as the fact that they don't use the new
`bevy_picking` framework, or the fact that they are really only useful for creating buttons
and not other kinds of widgets like sliders.

As an art form, games thrive on novelty: the typical game doesn't have boring, standardized controls
reminiscent of a productivity app, but instead will have beautiful, artistic widgets that are
in harmony with the game's overall visual theme. But writing new and unique widgets requires
skill and subtlety, particularly if we want first-class accessibility support. It's not a burden we
want to put on the average indie developer.

In the web development world, "headless" widget libraries, such as
[headlessui](https://headlessui.com/) and [reakit](https://reakit.io/) have become popular. These
provide standardized widgets that implement all of the correct interactions and behavioral logic,
including integration with screen readers, but which are unstyled. It's the responsibility of the
game developer to provide the visual style and animation for the widgets, which can fit the overall
style of their game.

With this release, Bevy introduces a collection of headless or "core" widgets. These are components
which can be added to any UI Node to get widget-like behavior. The core widget set includes buttons,
sliders, scrollbars, checkboxes, radio buttons, and more. This set will likely be expanded in
future releases.

## Widget Interaction States

Many of the core widgets will define supplementary ECS components that are used to store the widget's
state, similar to how the old `Interaction` component worked, but in a way that is more flexible.
These components include:

- `InteractionDisabled` - a marker component used to indicate that a component should be
  "grayed out" and non-interactive. Note that these disabled widgets are still visible and can
  have keyboard focus (otherwise the user would have no way to discover them).
- `Hovering` is a simple boolean component that allows detection of whether the widget is being
  hovered using regular Bevy change detection.
- `Checked` is a boolean component that stores the checked state of a checkbox or radio button.
- `ButtonPressed` is used for a button-like widget, and will be true while the button is held down.

The combination of `Hovering` and `ButtonPressed` fulfills the same purpose as the old `Interaction`
component, except that now we can also represent "roll-off" behavior (the state where you click
on a button and then, while holding the mouse down, move the pointer out of the button's bounds).
It also provides additional flexibility in cases where a widget has multiple hoverable parts,
or cases where a widget is hoverable but doesn't have a pressed state (such as a tree-view expansion
toggle).
