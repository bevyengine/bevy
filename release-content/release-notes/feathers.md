---
title: Bevy Feathers
authors: ["@viridia", "@Atlas16A", "@ickshonpe", "@amedoeyes"]
pull_requests: [19730, 19900, 19928, 20237, 20169, 20422, 20350, 20548, 20969]
---

TODO: add screenshot of `feathers` in practice

To make it easier for Bevy engine developers and third-party tool creators to make comfortable, visually cohesive tooling,
we're pleased to introduce "Feathers" - a comprehensive widget set that offers:

- Standard widgets designed to match the look and feel of the planned Bevy Editor
- Components that can be leveraged to build custom editors, inspectors, and utility interfaces
- Essential UI elements including buttons, sliders, checkboxes, menu buttons, and more
- Layout containers for organizing and structuring UI elements
- Decorative elements such as icons for visual enhancement
- Robust-if-primitive theming support ensuring consistent visual styling across applications
- Accessibility features with built-in screen reader and assistive technology support
- Interactive cursor behavior that changes appropriately when hovering over widgets
- A virtual keyboard suitable for touchscreen text input

Feathers isn't meant as a toolkit for building exciting and cool game UIs: it has a somewhat plain
and utilitarian look and feel suitable for editors and graphical utilities. That being said, using
the themeing framework, you can spice up the colors quite a bit.

This is still early in development, and is currently hidden behind an experimental feature flag:
`experimental_bevy_feathers`.
If you're looking to experiment with building tooling for Bevy, turn that on and use `feathers` as is!
Let us know what problems you run into, and feel free to contribute missing widgets and bugs upstream.

But if you can't wait to get your hands on `bevy_ui` widgets for your game,
copy the code into your project and start hacking away at it!
While it deliberately does not expose very many tuning levers (keeping a coherent visual style in an open source project is _hard_),
it's a helpful base to understand how to extend and style `bevy_ui` and our new headless widgets
to meet the unique style and design patterns of your project.
