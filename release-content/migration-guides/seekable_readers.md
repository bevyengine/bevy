---
title: Implementations of `Reader` now must implement `Reader::seekable`, and `AsyncSeekForward` is deleted.
pull_requests: [22182]
---

The `Reader` trait no longer requires implementing `AsyncSeekForward` and instead requires
implementing `Reader::seekable`, which will cast the `Reader` to `&mut dyn SeekableReader` if it
supports `AsyncSeek` (`SeekableReader: Reader + AsyncSeek`).

```rust
// If MyReader implements `AsyncSeek` 
impl Reader for MyReader {
    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Ok(self)
    }
}

// If MyReader does not implement `AsyncSeek` 
impl Reader for MyReader {
    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        None
    }
}
```

Since we now just use the `AsyncSeek` trait, we've deleted the `AsyncSeekForward` trait. Users of
this trait can migrate by calling the `AsyncSeek::poll_seek` method with
`SeekFrom::Current(offset)`, or the `AsyncSeekExt::seek` method.
