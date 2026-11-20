---
title: AssetProcessors now require an explicit name.
pull_requests: [26433]
---

In previous versions of Bevy, asset processors were named by their type name. This resulted in meta
files referencing processors like `LoadTransformAndSave<RonLoader, DoSomeStuff, RonSaver>`. This
conflates the "name" of the processor with its implementation, making it impossible to use the same
type for multiple processors (e.g., processor that change their behavior dynamically based on stored
values).

Now, processors must be explicitly named. So you can migrate your code like so:

```rust
// Before
app
    .register_asset_processor::<LoadTransformAndSave<RonLoader, DoSomeStuff, RonSaver>>(
        LoadTransformAndSave::new(DoSomeStuff::new(), RonSaver::new())
    )
    .set_default_processor::<LoadTransformAndSave<RonLoader, DoSomeStuff, RonSaver>>("thing.ron");

// After
app
    .register_asset_processor(
        "DoStuffToRon",
        LoadTransformAndSave::<RonLoader, _, _>::with_implicit_loader(
            DoSomeStuff::new(), RonSaver::new())
    )
    .set_default_processor("thing.ron", "DoStuffToRon");
```

In this example, we named the processor `DoStuffToRun`. In your meta files, you may now use
`DoStuffToRon` to reference your processor. Meta files can still use the short and long type name,
however this is deprecated and will be removed in 0.22.

Note: `register_asset_processor` can no longer use the turbo-fish operator. You will need to move
the processor type to the function argument.
