---
title: Extended material 2D
authors: ["@cookie1170"]
pull_requests: [25183]
---

Bevy now provides a 2D analog to 3D's [`ExtendedMaterial`], which can be used to extend an existing material by implementing the `MaterialExtension2d` trait:

```rs
#[derive(AsBindGroup, Reflect, Clone)]
struct MyExtendedMaterial {
    // Make sure to make this high enough to not collide with the base material's uniforms!
    #[uniform(20)]
    important_binding: Vec4,
}

impl MaterialExtension2d for MyExtendedMaterial {
    fn vertex_shader() -> Option<ShaderRef> {
        None // Return `Some` to override the base material's vertex shader
    }

    fn fragment_shader() -> Option<ShaderRef> {
        None // Return `Some` to override the base material's fragment shader
    }

    fn depth_bias(&self) -> Option<f32> {
        None // Return `Some` to override the base material's depth bias
    }

    fn alpha_mode(&self) -> Option<AlphaMode2d> {
        None // Return `Some` to override the base material's alpha mode
    }
}
```

This material can now be used in an `ExtendedMaterial2d` struct:

```rs
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // Make sure to add a plugin for the material!
            Material2dPlugin::<ExtendedMaterial2d<ColorMaterial, MyExtendedMaterial>>::default()
        ))
        .add_systems(Startup, spawn_extended_material_mesh)
        .run();
}

fn spawn_extended_material_mesh(
    mut commands: Commands,
    mut materials: ResMut<Assets<ExtendedMaterial2d<ColorMaterial, MyExtendedMaterial>>>,
) {
    // Create an extended material with a `ColorMaterial` as the base and `MyExtendedMaterial` as the extension
    // `ColorMaterial`'s bindings will be available to `MyExtendedMaterial`'s shader
    let material = ExtendedMaterial2d {
        base: ColorMaterial::from_color(Color::WHITE),
        extension: MyExtendedMaterial {
            important_binding: Vec4::ZERO,
        },
    };

    let handle = materials.add(material);
    commands.spawn((
        Mesh2d,
        MeshMaterial2d(handle),
    ));
}
```

[`ExtendedMaterial`]: https://docs.rs/bevy/latest/bevy/pbr/struct.ExtendedMaterial.html
