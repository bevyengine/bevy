use bevy::prelude::*;

// This example shows how to use a custom mesh with a `SpriteBundle`. It won't go into most
// of the details of the mesh creation, as those are covered by the example 2d/mesh.rs, which
// use a `MeshBundle`. The `SpriteBundle` has two more components:
// - `Handle<ColorMaterial>` used to specify either the color or the texture
// - `Sprite` used to set the size of the image
// These two components let us use the default pipeline instead of defining our own with
// a custom shader as can be seen in the mesh example, but are less flexible if you need
// to do custom things for the color

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(camera)
        // We will do two shapes: stars and discs, they are both simple meshes to generate
        .add_startup_system(stars)
        .add_startup_system(discs)
        .run();
}

fn camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

// Number of points the generated polygons will have
// For stars, the more points, the pointier the star will be. A 5 points star is a pentagram
// For discs, the more points, the roundier the disc will be. A 3 points disc is a triangle
const POINTS: [u32; 10] = [3, 4, 5, 6, 7, 8, 10, 15, 30, 50];

fn stars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    let material = materials.add(texture_handle.into());

    // We will do two lines of stars: one with a texture, the other with a plain color
    let with_image = SpriteBundle {
        // Use the material for the texture
        material: material.clone(),
        ..Default::default()
    };

    let with_plain_color = SpriteBundle {
        // Use the material for a plain color
        material: materials.add(Color::rgb(0.25, 0.25, 0.75).into()),
        ..Default::default()
    };

    for (line, basis) in [with_image, with_plain_color].iter().enumerate() {
        for (pos, star_points) in POINTS.iter().enumerate() {
            let mut star = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);

            let mut v_pos = vec![[0.0, 0.0, 0.0]];
            // The default pipeline needs two other mesh attributes:
            // - Normals - it can be ignored in 2d and just use `[0.0, 0.0, 1.0]` for every point
            // - UVs - it's the position of the 2d texture applied to the mesh. While
            // `[0.0, 0.0, 0.0]` is the center of the mesh, `[0.0, 0.0]` is the top left corner
            // of the texture
            let mut v_normals = vec![[0.0, 0.0, 1.0]];
            let mut v_uvs = vec![[0.5, 0.5]];

            for i in 0..(star_points * 2) {
                let a = std::f32::consts::FRAC_PI_2
                    - i as f32 * std::f32::consts::TAU / (star_points * 2) as f32;
                let r = if i % 2 == 0 { 1.0 } else { 0.4 };
                v_pos.push([r * a.cos(), r * a.sin(), 0.0]);
                // Just use the same normal for every point
                v_normals.push([0.0, 0.0, 1.0]);
                // Those UV values won't deform the original image, but will cut parts that are
                // out of the star.
                v_uvs.push([(r * a.cos() + 1.0) / 2.0, 1.0 - (r * a.sin() + 1.0) / 2.0]);
            }
            star.set_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);
            star.set_attribute(Mesh::ATTRIBUTE_NORMAL, v_normals);
            star.set_attribute(Mesh::ATTRIBUTE_UV_0, v_uvs);

            let mut indices = vec![0, 1, (star_points * 2)];
            for i in 2..=(star_points * 2) {
                indices.extend_from_slice(&[0, i, i - 1]);
            }
            star.set_indices(Some(bevy::render::mesh::Indices::U32(indices)));

            commands.spawn_bundle(SpriteBundle {
                mesh: meshes.add(star),
                sprite: Sprite::new(Vec2::new(50., 50.)),
                transform: Transform::from_xyz(
                    (pos as f32 - POINTS.len() as f32 / 2.0 + 0.5) * 110.0,
                    (line as f32 + 0.5) * 110.0,
                    0.0,
                ),
                ..basis.clone()
            });
        }
    }
}

fn discs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    let material = materials.add(texture_handle.into());

    let with_image = SpriteBundle {
        material: material.clone(),
        ..Default::default()
    };

    let with_plain_color = SpriteBundle {
        material: materials.add(Color::rgb(0.25, 0.75, 0.25).into()),
        ..Default::default()
    };

    for (line, basis) in [with_image, with_plain_color].iter().enumerate() {
        for (pos, disc_points) in POINTS.iter().enumerate() {
            let mut disc = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);

            let mut v_pos = vec![[0.0, 0.0, 0.0]];
            let mut v_normals = vec![[0.0, 0.0, 1.0]];
            let mut v_uvs = vec![[0.5, 0.5]];

            for i in 0..*disc_points {
                let a = std::f32::consts::FRAC_PI_2
                    - i as f32 * std::f32::consts::TAU / (*disc_points as f32);
                v_pos.push([a.cos(), a.sin(), 0.0]);
                v_normals.push([0.0, 0.0, 1.0]);
                v_uvs.push([(a.cos() + 1.0) / 2.0, 1.0 - (a.sin() + 1.0) / 2.0]);
            }
            disc.set_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);
            disc.set_attribute(Mesh::ATTRIBUTE_NORMAL, v_normals);
            disc.set_attribute(Mesh::ATTRIBUTE_UV_0, v_uvs);

            let mut indices = vec![0, 1, *disc_points];
            for i in 2..=*disc_points {
                indices.extend_from_slice(&[0, i, i - 1]);
            }
            disc.set_indices(Some(bevy::render::mesh::Indices::U32(indices)));

            commands.spawn_bundle(SpriteBundle {
                mesh: meshes.add(disc),
                sprite: Sprite::new(Vec2::new(50., 50.)),
                transform: Transform::from_xyz(
                    (pos as f32 - POINTS.len() as f32 / 2.0 + 0.5) * 110.0,
                    (line as f32 - 1.5) * 110.0,
                    0.0,
                ),
                ..basis.clone()
            });
        }
    }
}
