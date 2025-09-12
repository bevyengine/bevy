---
title: Headless Widgets
authors: ["@viridia", "@ickshonpe", "@alice-i-cecile"]
pull_requests: [19366, 19584, 19665, 19778, 19803, 20032, 20036, 20086, 20944]
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

With this release, Bevy introduces a collection of headless widgets. These are components
which can be added to any UI Node to get widget-like behavior. The standard widget set includes buttons,
sliders, scrollbars, checkboxes, radio buttons, and more. This set will likely be expanded in
future releases.

While these widgets are usable today, and are a solid choice for creating your own widgets for your
own game or application, they are still **experimental**.
Expect breaking changes as we continue to iterate and improve on them!

We're as excited as you are for first-party widgets,
and we've decided to ship these now precisely so people can try them out:
real-world user feedback is vital for building and improving products.

If you've read this and are still excited to try them out, enable the `experimental_bevy_ui_widgets` feature.

## Standard Widgets

The `bevy_ui_widgets` crate provides implementations of unstyled widgets, such as buttons,
sliders, checkboxes and radio buttons.

- `ui_widgets::Button` is a push button. It emits an activation event when clicked.
- `ui_widgets::Slider` is a standard slider, which lets you edit an `f32` value in a given range.
- `ui_widgets::Scrollbar` can be used to implement scrollbars.
- `ui_widgets::Checkbox` can be used for checkboxes and toggle switches.
- `ui_widgets::RadioButton` and `ui_widgets::RadioGroup` can be used for radio buttons.

## Widget Interaction Marker Components

Many of the standard widgets will define supplementary ECS components that are used to store the widget's
state, similar to how the old `Interaction` component worked, but in a way that is more flexible.
These components include:

- `InteractionDisabled` - a boolean component used to indicate that a component should be
  "grayed out" and non-interactive. Note that these disabled widgets are still visible and can
  have keyboard focus (otherwise the user would have no way to discover them).
- `Hovered` is a simple boolean component that allows detection of whether the widget is being
  hovered using regular Bevy change detection.
- `Checked` is a boolean component that stores the checked state of a checkbox or radio button.
- `Pressed` is used for a button-like widget, and will be true while the button is held down.

The combination of `Hovered` and `Pressed` fulfills the same purpose as the old
`Interaction` component, except that now we can also represent "roll-off" behavior (the state where
you click on a button and then, while holding the mouse down, move the pointer out of the button's
bounds). It also provides additional flexibility in cases where a widget has multiple hoverable
parts, or cases where a widget is hoverable but doesn't have a pressed state (such as a tree-view
expansion toggle).

## Widget Notifications

Applications need a way to be notified when the user interacts with a widget. One way to do this
is using Bevy observers. This approach is useful in cases where you want the widget notifications
to bubble up the hierarchy.

However, in UI work it's often desirable to send notifications "point-to-point" in ways that cut
across the hierarchy. For these kinds of situations, the standard widgets offer a different
approach: callbacks. The `Callback` enum allows different options for triggering a notification
when a widget's state is updated. For example, you can pass in the `SystemId` of a registered
one-shot system as a widget parameter when it is constructed. When the button subsequently
gets clicked or the slider is dragged, the system gets run. Because it's an ECS system, it can
inject any additional parameters it needs to update the Bevy world in response to the interaction.

## State Management

See the [Wikipedia Article on State Management](https://en.wikipedia.org/wiki/State_management).

Most of the standard widgets support "external state management" - something that is referred to in the
React.js world as "controlled" widgets. This means that for widgets that edit a parameter value
(such as checkboxes and sliders), the widget doesn't automatically update its own internal value,
but only sends a notification to the app telling it that the value needs to change. It's the
responsibility of the app to handle this notification and update the widget accordingly, and at the
same time update any other game state that is dependent on that parameter.

There are multiple reasons for this, but the main one is this: typical game user interfaces aren't
just passive forms of fields to fill in, but more often represent a dynamic view of live data. As a
consequence, the displayed value of a widget may change even when the user is not directly
interacting with that widget. Externalizing the state avoids the need for two-way data binding, and
instead allows simpler one-way data binding that aligns well with the traditional "Model / View /
Controller" (MVC) design pattern.

That being said, the choice of internal or external state management is up to you: if the widget has
an `on_change` callback that is not `Callback::Ignore`, then the callback is used. If the callback
is `Callback::Ignore`, however, the widget will update its own state automatically. (This is similar
to how React.js does it.)

There are two exceptions to this rule about external state management. First, widgets which don't
edit a value, but which merely trigger an event (such as buttons), don't fall under this rule.
Second, widgets which have complex states that are too large and heavyweight to fit within a
notification event (such as a text editor) can choose to manage their state internally. These latter
widgets will need to implement a two-way data binding strategy.
