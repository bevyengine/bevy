---
title: One-to-many Asset Processing
authors: ["@andriyDev"]
pull_requests: []
---

In previous versions, asset processing was always one-to-one: a processor would be given a single
asset to process and write to a single file.

Now, an asset processor can write to multiple files! When implementing the `Process` trait, you can
call `writer_context.write_multiple` and provide the path relative to the original asset. So for
example, here we have a processor that reads all the lines in a file and writes them each to their
own file:

```rust
struct LineSplitterProcess;

impl Process for LineSplitterProcess {
    type Settings = ();

    async fn process(
        &self,
        context: &mut ProcessContext<'_>,
        meta: AssetMeta<(), Self>,
        writer_context: WriterContext<'_>,
    ) -> Result<(), ProcessError> {
        let bytes = context.asset_bytes();
        if bytes.is_empty() {
            return Err(ProcessError::AssetTransformError("empty asset".into()));
        }
        for (i, line) in bytes.lines().map(Result::unwrap).enumerate() {
            let mut writer = writer_context
                .write_multiple(Path::new(&format!("Line{i}.line")))
                .await?;
            writer.write_all(line.as_bytes()).await.map_err(|err| {
                ProcessError::AssetWriterError {
                    path: context.path().clone_owned(),
                    err: err.into(),
                }
            })?;
            writer.finish::<TextLoader>(TextSettings::default()).await?;
        }
        Ok(())
    }
}
```

Then if you have an asset like `shakespeare.txt`, you can load these separate files as
`shakespeare.txt/Line0.line`, `shakespeare.txt/Line1.line`, etc. These separate files can have
different file extensions, be loaded as completely separate asset types, or be entirely produced
from scratch within the asset processor! These files are treated as completely distinct assets, so
loading them looks like a regular asset load (e.g.,
`asset_server.load("shakespeare.txt/Line1.line")`).

We plan to use this to break apart large glTF files into smaller, easier-to-load pieces -
particularly for producing virtual geometry meshes.
