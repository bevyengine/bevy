---
title: "`UnpreparedBindGroup` is now `BindGroupBuilder`"
pull_requests: [25058]
---

The `UnpreparedBindGroup` structure is now known as `BindGroupBuilder`, and `AsBindGroup::unprepared_bind_group` is now known as `AsBindGroup::build_bind_group`.

If you're using `#[derive(AsBindGroup)]` to provide the implementation of `AsBindGroup`, then you shouldn't need to do anything in order to migrate, as the implementation of that derive macro has been updated accordingly. However, if you manually implement `AsBindGroup`, you may need to rename `unprepared_bind_group` to `build_bind_group` and write `UnpreparedBindingResource`s to the new `output` parameter instead of returning a new `UnpreparedBindGroup`. Generally, the contents of that method can be identical other than using `UnpreparedBindingResource`s and having to write to the `output` parameter; the exception is that `UnpreparedBindingResource::Data` now no longer takes a vector itself and instead specifies byte ranges in the shared `BindGroupBuilder::data_buffer`.

The primary motivation for this change was to reduce allocations. This occurs in two ways. First, the asset preparation infrastructure can reuse a single `BindGroupBuilder` when multiple materials need to be prepared instead of calling the `unprepared_bind_group` method, which allocates, again and again. Second, any `UnpreparedBindingResource::Data` resources inside the `BindGroupBuilder` can now reference a single `Vec` (which is cleared instead of reallocated as materials are prepared) rather than having to allocate anew for each material.
