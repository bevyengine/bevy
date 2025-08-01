Code that previously relied on `Material` from `prelude` will need to add:

```rust
use bevy::pbr::Material;
```

Example before:

```rust
use bevy::prelude::*; // v0.16

struct MyAwesomeMaterial;

impl Material for MyAwesomeMaterial {
  // ...
}
```

After:

```rust
use bevy::prelude::*;
use bevy::pbr::Material; // v0.17

struct MyAwesomeMaterial;

impl Material for MyAwesomeMaterial {
  // ...
}
```