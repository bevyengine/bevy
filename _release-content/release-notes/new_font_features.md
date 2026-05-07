---
title: "Richer text"
authors: ["@ickshonpe", "@alice-i-cecile", "@gregcsokas"]
pull_requests: [22156, 22396, 22614, 22879, 23380]
---

TODO: screenshot showing generic families, weights, and responsive sizing in action

Bevy's text system has historically been sparse: pick a font by asset handle, set a size in pixels, done.
Want bold? Load a separate bold font asset.
Want italic? Another asset.
Want the user's system monospace? No luck.
Want text that scales with the viewport? Roll it yourself.

Not anymore.

## Better font selection

`FontSource` now offers three ways to identify a font:

```rust
// By asset handle — same behavior as before, now wrapped in FontSource
TextFont::default().with_font(asset_server.load("fonts/FiraMono.ttf"))

// By family name — resolved from the font database
TextFont { font: FontSource::Family("FiraMono".into()), ..default() }

// By semantic category
TextFont { font: FontSource::Monospace, ..default() }
```

The generic variants — `Serif`, `SansSerif`, `Cursive`, `Fantasy`, `Monospace`, and several UI-specific ones (`SystemUi`, `Emoji`, `Math`, and others) — resolve to configurable defaults. Override them via `FontCx`:

```rust
fn configure_fonts(mut font_cx: ResMut<FontCx>) {
    font_cx.set_serif_family("Merriweather");
    font_cx.set_monospace_family("JetBrains Mono");
}
```

Editor tooling and non-game applications that want to respect the user's font preferences without hardcoding an asset path will find this particularly useful.

System fonts were already loadable via the backend resource in previous releases, but `FontSource::Family` is a cleaner, more powerful way to load them.
Enable the `bevy/system_font_discovery` feature to make installed system fonts available by name; without it, `FontSource::Family("...")` will only find fonts explicitly loaded as Bevy assets.

## Variable font properties

`TextFont` has gained the `weight`, `width`, and `style` fields. Pick a variable font, and say goodbye to separate assets for every variant of a typeface:

```rust
TextFont {
    font: FontSource::SansSerif,
    weight: FontWeight::BOLD,
    style: FontStyle::Italic,
    width: FontWidth::CONDENSED,
    ..default()
}
```

`FontWeight` accepts any value from 1–1000. `FontStyle` is `Normal`, `Italic`, or `Oblique`.
`FontWidth` covers the full OpenType stretch range from `ULTRA_CONDENSED` (50%) to `ULTRA_EXPANDED` (200%).

## Responsive font sizing

`font_size` is now a `FontSize` enum rather than a bare `f32`:

```rust
TextFont::from_font_size(FontSize::Px(24.0))   // fixed pixels — unchanged behavior
TextFont::from_font_size(FontSize::Vh(5.0))    // 5% of viewport height
TextFont::from_font_size(FontSize::Rem(1.5))   // relative to the RemSize resource
```

The full set of variants mirrors CSS: `Px`, `Vw`, `Vh`, `VMin`, `VMax`, and `Rem`. `Rem` values scale with the `RemSize` resource, giving you a single knob to resize all relative text at once. Note that `Text2d` resolves viewport units against the primary window, not the render target — a deliberate compromise for entities that can render to multiple viewports.

## Letter spacing

A new `LetterSpacing` component controls the spacing between characters:

```rust
commands.spawn((
    Text::new("SPACED OUT"),
    LetterSpacing::Px(4.0),
));
```

It follows the same pattern as `LineHeight`, so negative values bring characters closer together. Note that LetterSpacing currently only supports `Px` — `Rem` support remains planned.

While all of these features would have been possible in [`cosmic_text`],
we've chosen to migrate to [`parley`] during this cycle.
Both are solid, modern choices, but we found `parley` had meaningfully better documentation and was somewhat nicer to use.

[`cosmic_text`]: https://github.com/pop-os/cosmic-text
[`Parley`]: https://github.com/linebender/parley
