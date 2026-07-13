---
title: "Contextual theming"
authors: ["@viridia"]
pull_requests: [24969]
---

Feathers now supports "contextual theming", meaning that the theme variables can change depending
on the parent entity. So widgets that are inside of a dialog box or subpanel can have different
colors than wigets that are on a regular panel or window background.

The design follows that of popular web toolkits like MUI, Radix, or Chakra. There's a new component,
`ThemeContext`, which lets you select which color scheme the widget's descendants should use;
currently the available schemes are `Base`, `Higher`, `Highest`, and `Floating`, which correspond
to the design plans for the Bevy scene editor.

The theme context is used in conjunction with a new kind of design token, named `SemanticToken`.
The lookup process for a color now requires two stages: the `ThemeToken` is converted into a
`SemanticToken`, and then the combination of `SemanticToken` and `ThemeContext` is used to look up
a color.

In addition to allowing context-specific color choices, this also makes it easier to design new
themes! Instead of having to tediously choose colors for a hundred different theme tokens,
the set of semantic tokens is much smaller, and the relationship between token and color is much
more intuitive.
