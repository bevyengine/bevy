---
title: BSN Syntax Improvements.
pull_requests: [25318]
---

BSN landed with a few idiosyncrasies that caused friction in practice. We made some changes to BSN's syntax this cycle in the interest of improving its ergonomics and clarity.

All scene references now require `@` prefixes:

```rust
// Before
bsn! {
    scene_variable
    scene_function()
    {scene_expression}
}

// After
bsn! {
    @scene_variable
    @scene_function()
    @{scene_expression}
}
```

This freed us up to make component values _much_ easier to work with.

```rust
// Before
bsn! {
    template_value(component_variable)
    template_value(component_function())
}

// After
bsn! {
    component_variable
    component_function()
}
```

Enums no longer require `VariantDefaults` or `FromTemplate`, provided they implement `Default` and `Clone`:

```rust
// Before
#[derive(Component, Default, Clone, VariantDefaults)]
enum Foo {
    A { x: u32, y: u32 },
    #[default]
    B,
}

bsn! {
    Foo::B
}

// After
#[derive(Component, Default, Clone)]
enum Foo {
    A { x: u32, y: u32 },
    #[default]
    B,
}

bsn! {
    Foo::B
}
```

If you were using an enum that didn't support `VariantDefaults`, you can remove the `template_value` wrapper:

```rust
// Before
bsn! {
    template_value(Foo::A)
}
// After
bsn! {
    Foo::A
}
```

The "variant defaults" pattern, which relied on defining individual "default" constructors for each variant is what allowed "individual enum field value patching" (ex: `VariantDefaults` and `FromTemplate` would define `Foo::a_default()` and `Foo::b_default()` in the example above). This is no longer supported, as the weirdness factor (and Rust ecosystem compatibility challenges) were too costly. When working with enums in BSN, you must now specify each field in the enum, just like you would in normal Rust (which doesn't have support for individual enum variant defaults).

```rust
// Before (y field is initialized to its default value)
bsn! {
    Foo::A { x: 1 }
}

// After (y field must be manually specified)
bsn! {
    Foo::A { x: 1, y: 0 }
}
```

The "builder pattern" previously required a `template_value` wrapper. This can now be removed:

```rust
// Before
bsn! {
    template_value(Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y))
}
// After
bsn! {
    Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y)
}
```

Additionally, you can now remove the `template_value` wrapper in cases like this:

```rust
// Before
bsn! {
    template_value(node.clone())
}
// After
bsn! {
    node.clone()
}
```

In general, you should now be able to remove all `template_value` instances from your BSN declarations!
