//! A simple scene to demonstrate spawning a viewport widget. The example will demonstrate how to
//! pick entities visible in the widget's view.

use bevy::{
    image::{TextureFormatPixelInfo, Volume},
    picking::pointer::PointerInteraction,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
    ui::widget::ViewportNode,
    window::PrimaryWindow,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_systems(Startup, test)
        .add_systems(Update, draw_mesh_intersections)
        .run();
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Shape;

fn test(
    mut commands: Commands,
    window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a UI camera
    commands.spawn(Camera3d::default());

    // Set up an texture for the 3D camera to render to
    let window = window.single().unwrap();
    let window_size = window.physical_size();
    let size = Extent3d {
        width: window_size.x,
        height: window_size.y,
        ..default()
    };
    let format = TextureFormat::Bgra8UnormSrgb;
    let image = Image {
        data: Some(vec![0; size.volume() * format.pixel_size()]),
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    let image_handle = images.add(image);

    // Spawn the 3D camera
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                // Render this camera before our UI camera
                order: -1,
                target: RenderTarget::Image(image_handle.clone().into()),
                ..default()
            },
        ))
        .id();

    // Spawn something for the 3D camera to look at
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(5.0, 5.0, 5.0))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(0.0, 0.0, -10.0),
            Shape,
        ))
        // We can observe pointer events on our objects as normal, the
        // `bevy::ui::widgets::viewport_picking` system will take care of ensuring our viewport
        // clicks pass through
        .observe(on_drag_cuboid);

    // Spawn our viewport widget
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(50.0),
                left: Val::Px(50.0),
                width: Val::Px(200.0),
                height: Val::Px(200.0),
                border: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            BorderColor(Color::WHITE),
            ViewportNode::new(camera),
        ))
        .observe(on_drag_viewport);
}

fn on_drag_viewport(drag: Trigger<Pointer<Drag>>, mut node_query: Query<&mut Node>) {
    if matches!(drag.button, PointerButton::Secondary) {
        let mut node = node_query.get_mut(drag.target()).unwrap();

        if let (Val::Px(top), Val::Px(left)) = (node.top, node.left) {
            node.left = Val::Px(left + drag.delta.x);
            node.top = Val::Px(top + drag.delta.y);
        };
    }
}

fn on_drag_cuboid(drag: Trigger<Pointer<Drag>>, mut transform_query: Query<&mut Transform>) {
    if matches!(drag.button, PointerButton::Primary) {
        let mut transform = transform_query.get_mut(drag.target()).unwrap();
        transform.rotate_y(drag.delta.x * 0.02);
        transform.rotate_x(drag.delta.y * 0.02);
    }
}

fn draw_mesh_intersections(
    pointers: Query<&PointerInteraction>,
    untargetable: Query<Entity, Without<Shape>>,
    mut gizmos: Gizmos,
) {
    for (point, normal) in pointers
        .iter()
        .flat_map(|interaction| interaction.iter())
        .filter_map(|(entity, hit)| {
            if !untargetable.contains(*entity) {
                hit.position.zip(hit.normal)
            } else {
                None
            }
        })
    {
        gizmos.arrow(point, point + normal.normalize() * 0.5, Color::WHITE);
    }
}
