---
title: AssetLoaders are no longer chosen by the asset type.
pull_requests: []
---

Previously, when picking which asset loader to use, the first step was looking up the asset loader
by the requested type. So if you loaded `asset_server.load::<Image>("blah.mp4")`, it would attempt
to load this `mp4` file with the `ImageLoader` (despite the fact that `mp4` is not a valid image
loader extension). This also lead to very complicated internal heuristics to deal with the fact that
a single asset file could be loaded as multiple different asset types at once.

Now, the asset loader selection can only use the file extension. This means for any file path there
is an unambiguous default loader.

This however breaks some uses. The most common is using a generic extension and then providing
asset loaders for those particular asset types. So for example, you may have files:

```
level1.ron
monster_snake.ron
```

Previously, if you had a `RonLoader<LevelDefinition>` and `RonLoader<Monster>`, these could be
loaded with `asset_server.load::<LevelDefinition>("level1.ron")` and
`asset_server.load::<Monster>("monster_snake.ron")` respectively, since the type of the `load` call
tells the asset system which loader to use.

Now, these two would conflict (and we'd use whichever loader was registered last). To resolve this,
one thing to do is to give a unique extension. A good pattern is to add the type name as the
extension, for example:

```
level1.LevelDefinition.ron
monster_snake.Monster.ron
```

(don't forget to update the `extensions` method in your `AssetLoader`)

Another approach is to use meta files. Meta files allow you to explicitly say which loader to use.
For example, we could write the following meta file at `level1.ron.meta`:

```
(
    meta_format_version: "1.0",
    asset: Load(
        loader: "RonLoader<LevelDefinition>",
        settings: (),
    ),
)
```

If you truly need to load one file with two loaders, come chat with us so we can better understand
your use-case!
