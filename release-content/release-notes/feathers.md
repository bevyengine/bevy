---
title: Bevy Feathers
authors: ["@viridia", "@Atlas16A", "@ickshonpe", "@amedoeyes"]
pull_requests: [19730, 19900, 19928, 20237, 20169, 20422, 20350, 20548, 20969]
---

TODO: add screenshot of `feathers` in practice

To make it easier for Bevy engine developers and third-party tool creators to make comfortable, visually cohesive tooling,
we're pleased to introduce "Feathers" - a comprehensive Bevy UI widget set. Feathers is meant to be Bevy's "developer tools" widget set, and it will be used to build the upcoming [Bevy Editor](https://bevy.org/news/bevys-fifth-birthday/#bevy-editor-design). It has a utilitarian look and feel, with an opinionated feature-set tailored for editors and graphical utilities. It builds on top of Bevy's new general-purpose "headless" widget set: `bevy_ui_widgets`. Feathers _can_ be used in games, but that is not its motivating use case.

Feathers currently offers:

- Standard widgets designed to match the look and feel of the planned Bevy Editor.
- Components that can be leveraged to build custom editors, inspectors, and utility interfaces that feel consistent with other Bevy tooling.
- Essential UI elements including buttons, sliders, checkboxes, menu buttons, and more.
- Layout containers for organizing and structuring UI elements.
- Decorative elements such as icons for visual enhancement.
- Initial (simple / primitive) theme support to ensure consistent, configurable visual styling across applications. This is not the "final" Bevy UI theme system, but it provides some baseline functionality.
- Accessibility features with built-in screen reader and assistive technology support.
- Interactive cursor behavior that changes appropriately when hovering over widgets.
- A virtual keyboard suitable for touchscreen text input.

Feathers is still early in development. It is currently hidden behind the `experimental_bevy_feathers` feature flag. Feathers is still incomplete and likely to change in a variety of ways:

- We will port Feathers to BSN (Bevy's [Next-Generation Scene/UI System](https://github.com/bevyengine/bevy/pull/20158/)) when that lands (targeted for **Bevy 0.18**).
- We are still discussing the best way to handle UI callbacks / events in Feathers. It includes a proposal API, but the debate is ongoing!
- We are still working on polishing up some UX issues.
- There are missing widgets and features. Notably the "text input" widget is still being developed.

If you're looking to experiment with building tooling for Bevy, enable it and take `feathers` for a test flight!
Let us know what problems you run into, and feel free to contribute missing widgets and bugs upstream.

If you can't wait to get your hands on `bevy_ui` widgets for your game,
we recommend copying the Feathers code into your project and start hacking away at it!
Feathers can serve as a helpful base to understand how to build and theme widgets in Bevy UI. It also illustrates how to use our new "headless" widget set: `bevy_ui_widgets`.
