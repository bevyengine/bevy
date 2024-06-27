//! Demonstrates how to create a mirror.

use std::{array, f32::consts::FRAC_PI_2};

use bevy::{
    color::palettes::css::{GOLDENROD, RED},
    math::{bounding::Aabb2d, reflection_matrix, uvec2, vec3, vec4},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        camera::CameraProjection,
        render_resource::{
            AsBindGroup, Extent3d, ShaderRef, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages,
        },
    },
};

// TODO: we'll use this to handle window resizes
#[derive(Resource)]
struct MirrorImage(Handle<Image>);

#[derive(Clone, AsBindGroup, Asset, Reflect)]
struct ScreenSpaceTextureExtension {
    #[uniform(100)]
    screen_rect: Vec4,
}

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.0,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, ScreenSpaceTextureExtension>,
        >::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut screen_space_texture_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, ScreenSpaceTextureExtension>>,
    >,
    mut images: ResMut<Assets<Image>>,
    _window: Query<&Window>,
) {
    // Spawn the main and mirror cameras.
    let camera_origin = vec3(-200.0, 200.0, -200.0);
    let camera_target = vec3(-25.0, 20.0, 0.0);
    let camera_transform =
        Transform::from_translation(camera_origin).looking_at(camera_target, Vec3::Y);

    let mirror_rotation = Quat::from_rotation_x(-FRAC_PI_2);
    let mirror_transform = Transform::from_scale(vec3(300.0, 1.0, 150.0))
        .with_rotation(mirror_rotation)
        .with_translation(vec3(-25.0, 75.0, 0.0));

    let proj = PerspectiveProjection::from_frustum_bounds(-0.1..0.1, -0.1..0.1, 0.1..1000.0);
    let mvp = proj.get_clip_from_view()
        * camera_transform.compute_matrix().inverse()
        * mirror_transform.compute_matrix();
    let plane_bounds = [
        mvp * vec4(-0.5, 0.0, -0.5, 1.0),
        mvp * vec4(-0.5, 0.0, 0.5, 1.0),
        mvp * vec4(0.5, 0.0, -0.5, 1.0),
        mvp * vec4(0.5, 0.0, 0.5, 1.0),
    ];
    let proj_plane_bounds: [Vec2; 4] =
        array::from_fn(|i| (plane_bounds[i].xyz() / plane_bounds[i].w).xy());

    // Main camera
    commands.spawn(Camera3dBundle {
        transform: camera_transform,
        projection: Projection::Perspective(proj),
        ..default()
    });

    // Householder matrix stuff

    // P C'^-1 M == P C^-1 H M
    // C'^1 == C^-1 H
    // C' == (C^-1 H)^-1
    // C' == H^-1 C

    // NB: This must be calculated in matrix form and then converted to a
    // transform! Transforms aren't powerful enough to correctly multiply
    // non-uniform scale, which `reflection_matrix` generates, by themselves.
    let reflected_transform = Transform::from_matrix(
        Mat4::from_mat3a(reflection_matrix(Vec3::NEG_Z)) * camera_transform.compute_matrix(),
    );

    let inverse_linear_camera_transform = camera_transform.compute_affine().matrix3.inverse();
    let mirror_near_plane_dist =
        Ray3d::new(camera_origin, (camera_target - camera_origin).normalize())
            .intersect_plane(Vec3::ZERO, InfinitePlane3d::new(mirror_rotation * Vec3::Y))
            .expect("Ray missed mirror");

    // Y=-1 because the near plane is the opposite plane of the mirror.
    let mirror_proj_plane_normal =
        (inverse_linear_camera_transform * (mirror_rotation * Vec3::NEG_Y)).normalize();

    let mirror_proj_near_dist = mirror_near_plane_dist - 0.1;

    // FIXME: This needs to be the actual window size, and should listen to
    // resize events and resize the texture as necessary
    let window_size = uvec2(1920, 1080);

    // Calculate the projected boundaries of the mirror on screen so that we can
    // allocate a texture of exactly the appropriate size.
    //
    // In reality you'll rarely want to do this, since reallocating textures is
    // expensive and portals can in general take up an arbitrarily large portion
    // of the screen. However, if the on-screen size of the portal is known to
    // be bounded, it can be useful to allocate a smaller texture, so we
    // demonstrate the technique here.
    let aabb_2d = Aabb2d::from_point_cloud(Vec2::ZERO, Rot2::IDENTITY, &proj_plane_bounds);
    let screen_space_aabb_2d = (vec4(aabb_2d.min.x, aabb_2d.max.y, aabb_2d.max.x, aabb_2d.min.y)
        * vec4(0.5, -0.5, 0.5, -0.5)
        + 0.5)
        * window_size.as_vec2().xyxy();
    let rounded_screen_space_aabb_2d = screen_space_aabb_2d
        .xy()
        .floor()
        .extend(screen_space_aabb_2d.z.ceil())
        .extend(screen_space_aabb_2d.w.ceil());
    let near_plane_aabb_2d = Aabb2d {
        min: aabb_2d.min * mirror_proj_near_dist,
        max: aabb_2d.max * mirror_proj_near_dist,
    };

    let mirror_image_size =
        (rounded_screen_space_aabb_2d.zw() - rounded_screen_space_aabb_2d.xy()).as_uvec2();

    let mirror_image_extent = Extent3d {
        width: mirror_image_size.x,
        height: mirror_image_size.y,
        depth_or_array_layers: 1,
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("mirror image"),
            size: mirror_image_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(mirror_image_extent);
    let image = images.add(image);
    commands.insert_resource(MirrorImage(image.clone()));

    let mut mirror_proj = PerspectiveProjection::from_frustum_bounds(
        near_plane_aabb_2d.min.x..near_plane_aabb_2d.max.x,
        near_plane_aabb_2d.min.y..near_plane_aabb_2d.max.y,
        mirror_proj_near_dist..1000.0,
    );
    mirror_proj.near_normal = mirror_proj_plane_normal;

    // Mirror camera
    commands.spawn(Camera3dBundle {
        camera: Camera {
            order: -1,
            target: image.clone().into(),
            // Reflecting the model across the mirror will flip the winding of
            // all the polygons. Therefore, in order to properly backface cull,
            // we need to turn on `invert_culling`.
            invert_culling: true,
            ..default()
        },
        transform: reflected_transform,
        projection: Projection::Perspective(mirror_proj),
        ..default()
    });

    // can't use a fox anymore because of https://github.com/bevyengine/bevy/issues/13796
    /*commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb")),
        transform: Transform::from_xyz(-50.0, 0.0, -100.0),
        ..default()
    });*/
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh()),
        transform: Transform::from_scale(vec3(80.0, 80.0, 80.0))
            .with_translation(vec3(-50.0, 0.0, -100.0)),
        material: standard_materials.add(Color::from(GOLDENROD)),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh()),
        material: standard_materials.add(Color::from(RED)),
        transform: Transform::from_xyz(10.0, 0.0, 10.0).with_scale(Vec3::splat(10.0)),
        ..default()
    });

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        material: screen_space_texture_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color_texture: Some(image),
                ..default()
            },
            extension: ScreenSpaceTextureExtension {
                screen_rect: rounded_screen_space_aabb_2d,
            },
        }),
        transform: mirror_transform,
        ..default()
    });
}

impl MaterialExtension for ScreenSpaceTextureExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/screen_space_texture_material.wgsl".into()
    }
}
