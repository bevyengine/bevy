---
title: Changes to the `Process` trait in `bevy_asset`.
pull_requests: [21925, 21889]
---

In previous versions, the `Process` trait had an associated type for the output loader, and took a
`writer` argument. This is no longer the case (in order to support one-to-many asset processing).
This change requires that users indicate whether they are using the "full" writer mode or the
"partial" writer mode.

If your previous trait implementation of `Process` looked like this:

```rust
impl Process for MyThing {
    type Settings = MyThingSettings;
    type OutputLoader = SomeLoader;

    async fn process(
        &self,
        context: &mut ProcessContext<'_>,
        meta: AssetMeta<(), Self>,
        writer: &mut Writer,
    ) -> Result<SomeLoader::Settings, ProcessError> {
        // Write to `writer`, then return the meta file you want.
        let meta = todo!();
        Ok(meta)
    }
}
```

Now it needs to look like:

```rust
impl Process for MyThing {
    type Settings = MyThingSettings;

    async fn process(
        &self,
        context: &mut ProcessContext<'_>,
        meta: AssetMeta<(), Self>,
        writer_context: WriteContext<'_>,
    ) -> Result<(), ProcessError> {
        let writer = writer_context.write_full().await?;
        // Write to `writer`, then return the meta file you want.
        let meta = todo!();
        writer.finish(meta).await
    }
}
```

In addition, the returned `writer` is a wrapper around `&mut Writer`. This means you may need to
explicitly dereference in order to pass this writer into functions that take a `&mut Writer`.

This does not apply if you are using the `LoadTransformAndSave` process - existing uses should
continue to work.

The `ProcessContext` also no longer includes `asset_bytes`. This has been replaced by
`asset_reader`. To maintain current behavior in a `Process` implementation, you can read all the
bytes into memory. If previously, you did:

```rust
// Inside `impl Process for Type`
let bytes = context.asset_bytes();
// Use bytes here!
```

Then now, it should be:

```rust
// Inside `impl Process for Type`
let reader = context.asset_reader();
let mut bytes = vec![];
reader
    .read_to_end(&mut bytes)
    .await
    .map_err(|err| ProcessError::AssetReaderError {
        path: context.path().clone_owned(),
        err: err.into(),
    })?;
// Use bytes here!
```
