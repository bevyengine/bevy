# Bevy Ptr

The `bevy_ptr` crate provides low-level abstractions for working with pointers in a more safe way than using rust's raw pointers.

Rust has lifetimed and typed references (`&'a T`), unlifetimed and typed references (`*const T`), but no lifetimed but untyped references.
`bevy_ptr` adds them, called `Ptr<'a>`, `PtrMut<'a>` and `OwningPtr<'a>`.
These types are lifetime-checked so can never lead to problems like use-after-frees and must always point to valid data.
