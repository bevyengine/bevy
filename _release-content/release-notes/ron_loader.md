---
title: RonLoader and RonSaver
authors: ["@andriyDev"]
pull_requests: [25754, 25770]
---

Custom assets are very useful to create. They allow users to define game-specific data that can be
shared across your entities. Unfortunately, to take full advantage of this, you need to define an
`AssetLoader` for your type. This can be cumbersome, since for most types, you just want to save and
load some plain old data! Previously, we didn't have an out-of-the-box way to do this.

Now, we provide `RonSaver` and `RonLoader` as ready-to-use asset savers/loaders. These types support
**any** reflected asset. `RonLoader` is able to load `.ron` files and reads the desired type from
inside the file (in other words, its format is self-documenting), and `RonSaver` exists to write
these kinds of files.

```rust
// Register the asset loader.
app.init_asset_loader::<RonLoader>();

let asset_server = app.world().resource::<AssetServer>().clone();
// Load the data from the "my_data.ron". We need the "#Typed" subasset, since the root asset for the
// RON can hold any type.
let my_data = asset_server.load::<MyData>("my_data.ron#Typed");
```

Here is an example of a RON file that can be loaded by `RonLoader` (this loader supports handles!):

```ron
{
  "MyData": (
    first_field: "abc",
    second_field: 10,
    handle_field: Path("some_other_path.gltf")
  )
}
```

In some cases though, this self-documenting behavior may be undesirable (you may actually care about
the format of the written data). For these cases, we also provide `TypedRonSaver` and
`TypedRonLoader`. These support any type that implements `Serialize` and `Deserialize`, and they are
serialized as normal RON data (no extra fluff). You also don't need the `#Typed` suffix when loading
the data. This however means that it is up to the user to load their data with the correct loader,
either by using a unique extension, setting an explicit loader in the meta file, or always loading
with the correct `T` when calling `AssetServer::load`. Here is what the RON file looks like:

```ron
(
  first_field: "abc",
  second_field: 10,
  # Handles don't impl Serialize or Deserialize, so we can't have a handle. Use RonLoader instead!
  # handle_field: Path("some_other_path.gltf")
)
```

Consider using these new loaders in place of your own bespoke loaders! In general, `TypedRonLoader`
will be the most direct replacement, while `RonLoader` is more featureful going forward.
