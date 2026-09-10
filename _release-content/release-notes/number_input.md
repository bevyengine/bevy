---
title: "Add scrubbing / dragging to number_input widget"
authors: ["@viridia"]
pull_requests: [24636, 24701]
---

The `FeathersNumberInput` widget has been substantially overhauled, with several new features.

Blender's [numeric input](https://docs.blender.org/manual/en/latest/interface/controls/buttons/fields.html)
is great, and we've borrowed its best elements. This includes support for multiple
editing modes — including "scrubbing" (click-and-drag) and direct keyboard
entry. The updated feathers widget is now much closer to feature parity with Blender.

The widget supports editing numbers of different data types: `f32`, `f64`, `i32` and `i64`.

The behavior of the widget can be configured through the use of several optional components:

- `HardLimit` specifies the minimum and maximum range for the value. If this component is absent,
  then the natural range of the data type is used.
- `SoftLimit` specifies the range that is accessible via dragging. Numbers that are entered by
  typing can exceed this limit.
- `NumberInputPrecision` is used to specify the number of decimal points of precision when dragging,
  so that you don't get a bunch of digits jumping around. This only quantizes the value when
  dragging, not when typing.
- `Step` is used to indicate the delta value when incrementing and decrementing.

When `SoftLimit` is present, the widget will look and feel like a slider: it will draw a slide
bar in the background, and the drag speed will be calculated such that changes in the bar's size
will be synchronized with movement of the mouse.

If `SoftLimit` is _not_ present, then the widget behaves more like a "scrubber", where there is
no slide bar, and drag speed is calculated based on a heuristic that takes into account precision,
step, and the current input value.

In either of this cases, a non-drag click event will activate "typing" mode, where a value can
be entered by typing digits.

Like all feathers widgets, this is a "controlled" widget, which means that the internal numeric
value is not automatically updated, but instead relies on the application's event handlers to
update the widget state in response to `ValueChange` events. Check out the `feathers_number_input`
example to see how to write such a handler trivially.
