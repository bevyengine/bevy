---
title: The `Process` trait no longer has a single output.
pull_requests: []
---

In previous versions, the `Process` trait had an associated type for the output loader, and took a
`writer` argument. This is no longer the case (in order to support one-to-many asset processing).
This change requires that users indicate whether they are using the "single" writer mode or the
"multiple" writer mode.

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
        let writer = writer_context.write_single().await?;
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
