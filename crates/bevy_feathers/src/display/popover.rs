//! Feathers visual styling for popover callout arrows.

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Asset, Assets, Handle};
use bevy_color::ColorToComponents;
use bevy_ecs::{prelude::*, VariantDefaults};
use bevy_math::{Affine2, Vec2, Vec4};
use bevy_picking::Pickable;
use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bevy_render::render_resource::AsBindGroup;
use bevy_scene::prelude::*;
use bevy_shader::ShaderRef;
use bevy_ui::{
    px, BackgroundColor, BorderColor, ComputedNode, Node, PositionType, UiGlobalTransform,
    UiSystems, ZIndex,
};
use bevy_ui_render::{
    ui_material::{MaterialNode, UiMaterial},
    UiMaterialPlugin,
};
use bevy_ui_widgets::popover::{
    Popover, PopoverArrow, PopoverDirection, PopoverPlugin, PopoverSide,
};

const ARROW_WIDTH: f32 = 16.0;
const ARROW_HEIGHT: f32 = 8.0;
const ARROW_EDGE_MARGIN: f32 = 4.0;
const ARROW_BORDER_ANTIALIAS_OVERLAP: f32 = 1.0;

/// Marks the triangle child of a [`FeathersPopoverArrow`].
#[derive(Component, Clone, Default)]
struct FeathersPopoverArrowVisual;

/// GPU material for the visible triangle of a [`FeathersPopoverArrow`].
///
/// Each arrow receives its own material because its colors and border width are
/// inherited from the edge of its direct parent [`Popover`].
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone, Default, PartialEq)]
struct PopoverArrowMaterial {
    /// Parent popover background color.
    #[uniform(0)]
    fill_color: Vec4,
    /// Color of the border that meets the arrow.
    #[uniform(1)]
    border_color: Vec4,
    /// Width of that border in physical pixels.
    #[uniform(2)]
    border_width: Vec4,
}

impl UiMaterial for PopoverArrowMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_feathers/assets/shaders/popover_arrow.wgsl".into()
    }
}

/// Identifies one of the two replacement border segments beside the arrow.
///
/// The parent popover's connecting border edge is suppressed and redrawn as
/// two segments, leaving a gap where the triangle joins the surface.
#[derive(Component, Clone, Copy, Default, VariantDefaults)]
#[require(BackgroundColor)]
enum FeathersPopoverArrowBorderSegment {
    #[default]
    Start,
    End,
}

/// A themed callout arrow for a [`bevy_ui_widgets::popover::Popover`].
///
/// Add this as a direct child of the popover. [`PopoverArrow`] positions and
/// rotates the zero-sized root. This scene adds a triangle styled to match the
/// parent popover.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersPopoverArrow;

impl FeathersPopoverArrow {
    fn scene() -> impl Scene {
        bsn! {
            PopoverArrow {
                corner_margin: { ARROW_WIDTH * 0.5 + ARROW_EDGE_MARGIN },
            }
            Pickable::IGNORE
            ZIndex(-1)
            Children [
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(-ARROW_WIDTH * 0.5),
                        top: px(-ARROW_HEIGHT),
                        width: px(ARROW_WIDTH),
                        height: px(ARROW_HEIGHT),
                    }
                    Pickable::IGNORE
                    MaterialNode::<PopoverArrowMaterial>(Handle::default())
                    FeathersPopoverArrowVisual
                ),
                (
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(0),
                        height: px(0),
                    }
                    Pickable::IGNORE
                    FeathersPopoverArrowBorderSegment::Start
                ),
                (
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(0),
                        height: px(0),
                    }
                    Pickable::IGNORE
                    FeathersPopoverArrowBorderSegment::End
                ),
            ]
        }
    }
}

fn initialize_arrow_materials(
    mut visuals: Query<&mut MaterialNode<PopoverArrowMaterial>, Added<FeathersPopoverArrowVisual>>,
    mut materials: ResMut<Assets<PopoverArrowMaterial>>,
) {
    for mut material in &mut visuals {
        material.0 = materials.add(PopoverArrowMaterial::default());
    }
}

/// Matches an arrow to its parent [`Popover`].
///
/// [`PopoverPlugin`] places the arrow and updates [`PopoverDirection`]. This
/// system copies the popover colors to the triangle and replaces the border
/// behind the arrow with two pieces so the join has no gap.
fn sync_arrow_style(
    arrows: Query<
        (&ChildOf, &Children, &PopoverDirection, &UiGlobalTransform),
        (
            With<FeathersPopoverArrow>,
            Without<FeathersPopoverArrowBorderSegment>,
        ),
    >,
    mut popovers: Query<
        (
            &mut ComputedNode,
            &UiGlobalTransform,
            Option<&BackgroundColor>,
            Option<&BorderColor>,
        ),
        (
            With<Popover>,
            Without<FeathersPopoverArrowBorderSegment>,
            Without<FeathersPopoverArrowVisual>,
        ),
    >,
    visuals: Query<&MaterialNode<PopoverArrowMaterial>, With<FeathersPopoverArrowVisual>>,
    mut materials: ResMut<Assets<PopoverArrowMaterial>>,
    mut segments: Query<
        (
            &FeathersPopoverArrowBorderSegment,
            &mut ComputedNode,
            &mut UiGlobalTransform,
            &mut BackgroundColor,
        ),
        Without<FeathersPopoverArrowVisual>,
    >,
) {
    for (parent, children, direction, arrow_transform) in &arrows {
        let Ok((mut computed_popover, popover_transform, popover_background, popover_border)) =
            popovers.get_mut(parent.parent())
        else {
            continue;
        };
        let popover_border = popover_border.copied().unwrap_or_default();
        let border_color = match direction.0 {
            PopoverSide::Top => popover_border.top,
            PopoverSide::Right => popover_border.right,
            PopoverSide::Bottom => popover_border.bottom,
            PopoverSide::Left => popover_border.left,
        };
        let background = popover_background.copied().unwrap_or_default();

        // Preserve the layout input while removing only the rendered edge for this frame.
        // The two segment nodes below redraw that edge with a gap for the arrow.
        let physical_border_width = remove_connecting_border(&mut computed_popover, direction.0);
        let arrow_material = PopoverArrowMaterial {
            fill_color: background.0.to_linear().to_vec4(),
            border_color: border_color.to_linear().to_vec4(),
            border_width: Vec4::splat(physical_border_width),
        };
        for child in children.iter() {
            let Ok(material_node) = visuals.get(child) else {
                continue;
            };
            let Some(mut material) = materials.get_mut(material_node) else {
                continue;
            };
            if *material != arrow_material {
                *material = arrow_material.clone();
            }
        }
        let Some(popover_inverse) = popover_transform.try_inverse() else {
            continue;
        };
        let arrow_local = popover_inverse.transform_point2(arrow_transform.translation);
        let segment_ranges = border_segment_ranges(
            &computed_popover,
            direction.0,
            arrow_local,
            physical_border_width,
        );

        for child in children.iter() {
            let Ok((segment, mut computed_segment, mut segment_transform, mut segment_background)) =
                segments.get_mut(child)
            else {
                continue;
            };
            let range = match segment {
                FeathersPopoverArrowBorderSegment::Start => segment_ranges.0,
                FeathersPopoverArrowBorderSegment::End => segment_ranges.1,
            };
            let (size, center) =
                segment_geometry(&computed_popover, direction.0, range, physical_border_width);
            update_segment_geometry(
                &mut computed_segment,
                &mut segment_transform,
                popover_transform,
                size,
                center,
            );
            let segment_color = BackgroundColor(border_color);
            if *segment_background != segment_color {
                *segment_background = segment_color;
            }
        }
    }
}

fn remove_connecting_border(popover: &mut ComputedNode, side: PopoverSide) -> f32 {
    let border_width = match side {
        PopoverSide::Top => &mut popover.border.min_inset.y,
        PopoverSide::Right => &mut popover.border.max_inset.x,
        PopoverSide::Bottom => &mut popover.border.max_inset.y,
        PopoverSide::Left => &mut popover.border.min_inset.x,
    };
    let width = *border_width;
    *border_width = 0.0;
    width.max(0.0)
}

fn border_segment_ranges(
    popover: &ComputedNode,
    side: PopoverSide,
    arrow_local: Vec2,
    border_width: f32,
) -> ((f32, f32), (f32, f32)) {
    let radius = popover.border_radius;
    let size = popover.size();
    let (side_min, side_max, arrow_axis) = match side {
        PopoverSide::Top => (
            -size.x * 0.5 + radius.top_left.x,
            size.x * 0.5 - radius.top_right.x,
            arrow_local.x,
        ),
        PopoverSide::Right => (
            -size.y * 0.5 + radius.top_right.y,
            size.y * 0.5 - radius.bottom_right.y,
            arrow_local.y,
        ),
        PopoverSide::Bottom => (
            -size.x * 0.5 + radius.bottom_left.x,
            size.x * 0.5 - radius.bottom_right.x,
            arrow_local.x,
        ),
        PopoverSide::Left => (
            -size.y * 0.5 + radius.top_left.y,
            size.y * 0.5 - radius.bottom_left.y,
            arrow_local.y,
        ),
    };
    let arrow_half_width = ARROW_WIDTH * 0.5 / popover.inverse_scale_factor;
    // Extend each segment beneath the arrow by one physical pixel. The
    // material's antialiased base corners are partially transparent, so an
    // exact edge-to-edge join otherwise exposes a small gap at either end.
    let overlap = border_width * 0.5 + ARROW_BORDER_ANTIALIAS_OVERLAP;
    let gap_min = (arrow_axis - arrow_half_width + overlap).clamp(side_min, side_max);
    let gap_max = (arrow_axis + arrow_half_width - overlap).clamp(side_min, side_max);
    ((side_min, gap_min), (gap_max, side_max))
}

fn segment_geometry(
    popover: &ComputedNode,
    side: PopoverSide,
    (min, max): (f32, f32),
    border_width: f32,
) -> (Vec2, Vec2) {
    let length = (max - min).max(0.0);
    let axis_center = (min + max) * 0.5;
    let half_size = popover.size() * 0.5;
    match side {
        PopoverSide::Top => (
            Vec2::new(length, border_width),
            Vec2::new(axis_center, -half_size.y + border_width * 0.5),
        ),
        PopoverSide::Right => (
            Vec2::new(border_width, length),
            Vec2::new(half_size.x - border_width * 0.5, axis_center),
        ),
        PopoverSide::Bottom => (
            Vec2::new(length, border_width),
            Vec2::new(axis_center, half_size.y - border_width * 0.5),
        ),
        PopoverSide::Left => (
            Vec2::new(border_width, length),
            Vec2::new(-half_size.x + border_width * 0.5, axis_center),
        ),
    }
}

fn update_segment_geometry(
    segment: &mut ComputedNode,
    transform: &mut UiGlobalTransform,
    popover_transform: &UiGlobalTransform,
    size: Vec2,
    local_center: Vec2,
) {
    if segment.size != size || segment.unrounded_size != size {
        segment.size = size;
        segment.unrounded_size = size;
    }
    let popover_affine = popover_transform.affine();
    let global = Affine2::from_mat2_translation(
        popover_affine.matrix2,
        popover_affine.transform_point2(local_center),
    );
    if transform.affine() != global {
        *transform = global.into();
    }
}

/// Plugin providing reusable Feathers visuals for popover callout arrows.
pub struct FeathersPopoverPlugin;

impl Plugin for FeathersPopoverPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }
        if !app.is_plugin_added::<UiMaterialPlugin<PopoverArrowMaterial>>() {
            app.add_plugins(UiMaterialPlugin::<PopoverArrowMaterial>::default());
        }
        app.add_systems(
            PostUpdate,
            (initialize_arrow_materials, sync_arrow_style)
                .chain()
                .after(crate::theme::update_theme)
                .after(UiSystems::Layout)
                .before(UiSystems::Stack),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_color::Color;
    use bevy_ecs::system::RunSystemOnce;
    use bevy_scene::ScenePlugin;
    use bevy_ui::UiRect;

    #[test]
    fn scene_builds_a_reusable_arrow() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
        ));

        let root = app
            .world_mut()
            .spawn_scene(bsn! { @FeathersPopoverArrow })
            .unwrap()
            .id();
        app.world_mut().flush();

        assert!(app.world().get::<FeathersPopoverArrow>(root).is_some());
        assert!(app.world().get::<PopoverArrow>(root).is_some());
        let node = app.world().get::<Node>(root).unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.width, px(0));
        assert_eq!(node.height, px(0));
        let children = app.world().get::<Children>(root).unwrap();
        assert_eq!(
            children
                .iter()
                .filter(|entity| app
                    .world()
                    .get::<FeathersPopoverArrowVisual>(*entity)
                    .is_some())
                .count(),
            1
        );
        assert_eq!(
            children
                .iter()
                .filter(|entity| app
                    .world()
                    .get::<FeathersPopoverArrowBorderSegment>(*entity)
                    .is_some())
                .count(),
            2
        );
        assert_eq!(app.world().get::<ZIndex>(root), Some(&ZIndex(-1)));
    }

    #[test]
    fn arrow_inherits_the_parent_surface_style() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
        ));
        app.init_asset::<PopoverArrowMaterial>();

        let background = BackgroundColor(Color::srgb(0.1, 0.2, 0.3));
        let border_color = Color::srgb(0.7, 0.8, 0.9);
        let parent = app
            .world_mut()
            .spawn((
                Node {
                    border: UiRect {
                        right: px(2),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Popover::default(),
                background,
                BorderColor {
                    right: border_color,
                    ..Default::default()
                },
            ))
            .id();
        let arrow = app
            .world_mut()
            .spawn_scene(bsn! { @FeathersPopoverArrow })
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(arrow)
            .insert((ChildOf(parent), PopoverDirection(PopoverSide::Right)));
        app.world_mut().flush();
        app.world_mut()
            .get_mut::<ComputedNode>(parent)
            .unwrap()
            .border
            .max_inset
            .x = 2.0;

        app.world_mut()
            .run_system_once(initialize_arrow_materials)
            .unwrap();
        app.world_mut().run_system_once(sync_arrow_style).unwrap();

        let visual = app
            .world()
            .get::<Children>(arrow)
            .unwrap()
            .iter()
            .find(|entity| {
                app.world()
                    .get::<FeathersPopoverArrowVisual>(*entity)
                    .is_some()
            })
            .unwrap();
        let material_node = app
            .world()
            .get::<MaterialNode<PopoverArrowMaterial>>(visual)
            .unwrap();
        let materials = app.world().resource::<Assets<PopoverArrowMaterial>>();
        assert_eq!(
            materials.get(material_node),
            Some(&PopoverArrowMaterial {
                fill_color: background.0.to_linear().to_vec4(),
                border_color: border_color.to_linear().to_vec4(),
                border_width: Vec4::splat(2.0),
            })
        );
        for child in app.world().get::<Children>(arrow).unwrap().iter() {
            if app
                .world()
                .get::<FeathersPopoverArrowBorderSegment>(child)
                .is_some()
            {
                assert_eq!(
                    app.world().get::<BackgroundColor>(child),
                    Some(&BackgroundColor(border_color))
                );
            }
        }
    }

    #[test]
    fn connecting_border_is_split_around_the_arrow() {
        let mut popover = ComputedNode::default();
        popover.size = Vec2::new(200.0, 100.0);
        popover.unrounded_size = popover.size;
        popover.inverse_scale_factor = 1.0;
        popover.border.max_inset.x = 2.0;
        popover.border_radius.top_right.y = 8.0;
        popover.border_radius.bottom_right.y = 8.0;

        let border_width = remove_connecting_border(&mut popover, PopoverSide::Right);
        assert_eq!(border_width, 2.0);
        assert_eq!(popover.border.max_inset.x, 0.0);

        let ranges = border_segment_ranges(
            &popover,
            PopoverSide::Right,
            Vec2::new(100.0, 10.0),
            border_width,
        );
        assert_eq!(ranges, ((-42.0, 4.0), (16.0, 42.0)));
        assert_eq!(
            segment_geometry(&popover, PopoverSide::Right, ranges.0, border_width),
            (Vec2::new(2.0, 46.0), Vec2::new(99.0, -19.0))
        );
        assert_eq!(
            segment_geometry(&popover, PopoverSide::Right, ranges.1, border_width),
            (Vec2::new(2.0, 26.0), Vec2::new(99.0, 29.0))
        );
    }

    #[test]
    fn connecting_border_split_covers_every_side_and_scale_factor() {
        for inverse_scale_factor in [1.0, 0.5] {
            for side in [
                PopoverSide::Top,
                PopoverSide::Right,
                PopoverSide::Bottom,
                PopoverSide::Left,
            ] {
                let mut popover = computed_node(Vec2::new(200.0, 100.0));
                popover.inverse_scale_factor = inverse_scale_factor;
                match side {
                    PopoverSide::Top => popover.border.min_inset.y = 2.0,
                    PopoverSide::Right => popover.border.max_inset.x = 2.0,
                    PopoverSide::Bottom => popover.border.max_inset.y = 2.0,
                    PopoverSide::Left => popover.border.min_inset.x = 2.0,
                }

                let border_width = remove_connecting_border(&mut popover, side);
                let ranges = border_segment_ranges(&popover, side, Vec2::ZERO, border_width);
                let side_extent = match side {
                    PopoverSide::Top | PopoverSide::Bottom => 100.0,
                    PopoverSide::Left | PopoverSide::Right => 50.0,
                };
                let arrow_half_width = ARROW_WIDTH * 0.5 / inverse_scale_factor;
                let overlap = border_width * 0.5 + ARROW_BORDER_ANTIALIAS_OVERLAP;

                assert_eq!(
                    ranges,
                    (
                        (-side_extent, -arrow_half_width + overlap),
                        (arrow_half_width - overlap, side_extent),
                    )
                );
            }
        }
    }

    fn computed_node(size: Vec2) -> ComputedNode {
        ComputedNode {
            size,
            unrounded_size: size,
            inverse_scale_factor: 1.0,
            ..Default::default()
        }
    }
}
