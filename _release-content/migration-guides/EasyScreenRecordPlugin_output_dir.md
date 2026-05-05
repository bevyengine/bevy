---
title: EasyScreenRecordPlugin output_dir
pull_requests: [23096]
---

`EasyScreenRecordPlugin` has a new public field `output_dir: Option<PathBuf>`.
If you are constructing this struct manually, you must now include the `output_dir` field.
If you are using `..default()`, no changes are needed.

```rust
// 0.18
let plugin = EasyScreenRecordPlugin {
    toggle: KeyCode::Space,
    preset: Preset::Medium,
    tune: Tune::Animation,
    frame_time: Duration::from_millis(33),
};

// 0.19
let plugin = EasyScreenRecordPlugin {
    toggle: KeyCode::Space,
    preset: Preset::Medium,
    tune: Tune::Animation,
    frame_time: Duration::from_millis(33),
    output_dir: Some("recordings".into()),
};
```
