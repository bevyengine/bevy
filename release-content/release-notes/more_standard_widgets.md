---
title: More Standard Widgets
authors: ["@viridia"]
pull_requests: [21636, 21743]
---

## More Standard Widgets

We are continuing to flesh out the collection of standard widgets first introduced in
Bevy 0.17.

### Popover

The `Popover` component can be placed on an absolutely-positioned UI node to provide
automatic popup positioning. This is inspired by the popular `floating-ui` npm package.

Popovers will be placed relative to an anchor element, and positioned so that they don't get
cut off by the window edge. You can specify a list of preferred "placements": top, bottom,
left or right, along with alignment options for each. If the popup is so large that it's
impossible to position it without it getting cut off, it will choose the placement that results
in the most visibility (as determined by the area cut off). (A future version might also
have an option to constrain the popup to be no larger than the window size, but this will be
more useful once we have better support for scrolling.)

This automatic positioning is dynamic, which means that if the anchor element moves around, is
inside a scrolling container, or the window is resized, the popover may "flip" sides in order to
remain fully visible.

Popovers can be used for dropdown menus, but they can also be used for tooltips.

### Menu

The `Menu` component uses `Popover` to provide a dropdown menu widget. This adds events for opening
and closing the menu, along with keyboard navigation and activation using the focus system.

### Color Plane

The `Color Plane` widget is a two-dimensional color picker that allows selecting two different
channels within a color space, one along the horizontal axis and one along the vertical. It can be
configured to display a variety of different color spaces: hue vs. lightness, hue vs. saturation,
red vs. blue, and so on.
