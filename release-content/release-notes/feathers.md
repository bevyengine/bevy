---
title: Bevy Feathers
authors: ["@viridia", "@Atlas16A", "@ickshonpe", "@amedoeyes"]
pull_requests: [19730, 19900, 19928, 20237, 20169, 20422, 20350, 20548]
---

To make it easier for Bevy engine developers and third-party tool creators to make comfortable, visually cohesive tooling,
we're pleased to introduce "Feathers" - a comprehensive widget set that offers:

- Standard widgets designed to match the look and feel of the planned Bevy Editor
- Components that can be leveraged to build custom editors, inspectors, and utility interfaces
- Essential UI elements including buttons, sliders, checkboxes, menu buttons, and more
- Layout containers for organizing and structuring UI elements
- Decorative elements such as icons for visual enhancement
- Robust theming support ensuring consistent visual styling across applications
- Accessibility features with built-in screen reader and assistive technology support
- Interactive cursor behavior that changes appropriately when hovering over widgets
- A virtual keyboard suitable for touchscreen text input

Feathers isn't meant as a toolkit for building exciting and cool game UIs: it has a somewhat plain
and utilitarian look and feel suitable for editors and graphical utilities. That being said, using
the themeing framework, you can spice up the colors quite a bit.
It can also serve as a helpful base to understand how to extend and style `bevy_ui` and our new core widgets;
copy the code into your project and start hacking!

Feathers is still in development, and is currently hidden behind an experimental feature flag,
`experimental_bevy_feathers`.
