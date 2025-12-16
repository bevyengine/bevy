---
title: OpenType Font Features
authors: ["@hansler"]
pull_requests: [19020]
---

OpenType font features allow fine-grained control over how text is displayed, including [ligatures](https://en.wikipedia.org/wiki/Ligature_(writing)), [small caps](https://en.wikipedia.org/wiki/Small_caps), and [many more](https://learn.microsoft.com/en-us/typography/opentype/spec/featurelist).

These features can now be used in Bevy, allowing users to add typographic polish (like discretionary ligatures and oldstyle numerals) to their UI. It also allows complex scripts like Arabic or Devanagari to render more correctly with their intended ligatures.

Example usage:

```rust
commands.spawn((
  TextSpan::new("Ligatures: ff, fi, fl, ffi, ffl"),
  TextFont {
    font: opentype_font_handle,
    font_features: FontFeatures::builder()
      .enable(FontFeatureTag::STANDARD_LIGATURES)
      .set(FontFeatureTag::WIDTH, 300)
      .build(),
    ..default()
  },
));
```

FontFeatures can also be constructed from a list:

```rust
TextFont {
  font: opentype_font_handle,
  font_features: [
    FontFeatureTag::STANDARD_LIGATURES,
    FontFeatureTag::STYLISTIC_ALTERNATES,
    FontFeatureTag::SLASHED_ZERO
  ].into(),
  ..default()
}
```

Note that OpenType font features are only available for `.otf` fonts that support them, and different fonts may support different subsets of OpenType features.
