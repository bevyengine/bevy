use bevy_app::App;
use bevy_app::HierarchyPropagatePlugin;
use bevy_app::PostUpdate;
use bevy_app::PropagateSet;
use bevy_app::TaskPoolPlugin;
use bevy_camera::Camera;
use bevy_camera::Camera2d;
use bevy_camera::ComputedCameraValues;
use bevy_camera::RenderTargetInfo;
use bevy_camera::Viewport;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::query::With;
use bevy_ecs::query::Without;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::utils::default;
use bevy_ecs::world::World;
use bevy_text::FontCx;
use bevy_ui::prelude::*;
use bevy_ui::sync_font_size_to_em_size;
use bevy_ui::ui_layout_system;
use bevy_ui::ui_surface::UiSurface;
use bevy_ui::update::propagate_ui_target_cameras;
use bevy_ui::update::update_clipping_system;
use criterion::criterion_group;
use criterion::BenchmarkId;
use criterion::Criterion;
use glam::UVec2;

criterion_group!(benches, layout);

struct Layout {
    roots: u32,
    nodes: u32,
    depth: u32,
}

impl Layout {
    fn label(&self) -> String {
        format!("{},{},{}", self.roots, self.nodes, self.depth)
    }
}

const LAYOUTS: [Layout; 5] = [
    Layout {
        roots: 1,
        nodes: 50,
        depth: 1,
    },
    Layout {
        roots: 1,
        nodes: 2,
        depth: 5,
    },
    Layout {
        roots: 1,
        nodes: 3,
        depth: 3,
    },
    Layout {
        roots: 4,
        nodes: 4,
        depth: 2,
    },
    Layout {
        roots: 10,
        nodes: 1,
        depth: 100,
    },
];

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default())
        .add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
            PostUpdate,
        ))
        .add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
            PostUpdate,
        ))
        .init_resource::<UiScale>()
        .init_resource::<UiSurface>()
        .init_resource::<FontCx>()
        .init_resource::<RemSize>()
        .add_systems(
            PostUpdate,
            (
                propagate_ui_target_cameras,
                sync_font_size_to_em_size,
                ui_layout_system,
                update_clipping_system,
            )
                .chain(),
        )
        .configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiTargetCamera>::default()
                .after(propagate_ui_target_cameras)
                .before(ui_layout_system),
        )
        .configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiRenderTargetInfo>::default()
                .after(propagate_ui_target_cameras)
                .before(ui_layout_system),
        );

    let size = UVec2::new(10000, 10000);
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: size,
                    scale_factor: 1.,
                }),
                ..Default::default()
            },
            viewport: Some(Viewport {
                physical_size: size,
                ..Default::default()
            }),
            ..Default::default()
        },
    ));
    app
}

fn spawn_layout(world: &mut World, is_root: bool, n: u32, d: u32) -> Entity {
    if d == 0 {
        return world
            .spawn(Node {
                width: px(1.),
                height: px(1.),
                ..default()
            })
            .id();
    }

    let sub_root = if is_root {
        world
            .spawn(Node {
                width: percent(100.),
                height: percent(100.),
                ..default()
            })
            .id()
    } else {
        world.spawn(Node::default()).id()
    };

    for _ in 0..n {
        let mut children = Vec::with_capacity(n as usize);
        for _ in 0..n {
            children.push(spawn_layout(world, false, n, d - 1));
        }
        world
            .spawn((Node::default(), ChildOf(sub_root)))
            .add_children(&children);
    }

    sub_root
}

fn layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("ui_layout");

    for layout in LAYOUTS {
        group.bench_function(BenchmarkId::new("static_layout", layout.label()), |b| {
            let mut app = setup_app();
            for _ in 0..layout.roots {
                spawn_layout(app.world_mut(), true, layout.nodes, layout.depth);
            }
            app.update();
            app.update();
            b.iter(|| app.update());
        });

        group.bench_function(BenchmarkId::new("update_roots", layout.label()), |b| {
            let mut app = setup_app();
            let mut roots = vec![];
            for _ in 0..layout.roots {
                roots.push(spawn_layout(
                    app.world_mut(),
                    true,
                    layout.nodes,
                    layout.depth,
                ));
            }
            app.update();
            app.update();
            b.iter(|| {
                for &root in &roots {
                    let mut node = app.world_mut().get_mut::<Node>(root).unwrap();
                    if node.justify_content == JustifyContent::Start {
                        node.justify_content = JustifyContent::End;
                    } else {
                        node.justify_content = JustifyContent::Start;
                    }
                }
                app.update();
            });
        });

        group.bench_function(BenchmarkId::new("update_leaves", layout.label()), |b| {
            let mut app = setup_app();
            for _ in 0..layout.roots {
                spawn_layout(app.world_mut(), true, layout.nodes, layout.depth);
            }
            let leaves: Vec<_> = app
                .world_mut()
                .query_filtered::<Entity, (With<Node>, Without<Children>)>()
                .iter(app.world())
                .collect();
            app.update();
            app.update();
            b.iter(|| {
                for &leaf in &leaves {
                    let mut node = app.world_mut().get_mut::<Node>(leaf).unwrap();
                    let length = if node.width == px(1) { px(2) } else { px(1) };
                    node.width = length;
                    node.height = length;
                }
                app.update();
            });
        });
    }
}
