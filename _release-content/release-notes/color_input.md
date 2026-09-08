---
title: "Feathers Color Input and Color Swatch Grid Widgets"
authors: ["@viridia"]
pull_requests: [25446]
---

Feathers has two new widgets for editing colors:

- `FeathersColorSwatchGrid` displays a 2D grid of clickable color swatches.
  Internally, these are radio buttons, so only one can be selected at a time.
  As radio buttons they support focus and keyboard navigation. The color values
  can be set from a Vec of colors, and if there are fewer colors in the Vec
  than there are slots in the grid, the extra slots will be filled with an
  empty cell placeholder.
- `FeathersColorInput` is a small button displaying a color swatch. When
  clicked, it brings up a drop-down menu displaying a color picker that supports
  both RGB and HSL picking modes. This widget can be placed anywhere that you
  want to display a color and make it editable. The widget also supports
  editing via hex, and displays a grid of recently-edited colors.
