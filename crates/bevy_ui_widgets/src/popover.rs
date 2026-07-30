//! Framework for positioning of popups, tooltips, and other popover UI elements.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Has, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{ParamSet, Query},
};
use bevy_math::{Affine2, Mat2, Rect, Rot2, Vec2};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    ui_layout_system, CalculatedClip, ComputedNode, ComputedUiRenderTargetInfo, Node, OverrideClip,
    PositionType, UiGlobalTransform, UiSystems, UiTransform, Val, Val2,
};

use crate::update_scrollbar_thumb;

/// Which side of the parent element the popover element should be placed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
pub enum PopoverSide {
    /// The popover element should be placed above the parent.
    Top,
    /// The popover element should be placed below the parent.
    #[default]
    Bottom,
    /// The popover element should be placed to the left of the parent.
    Left,
    /// The popover element should be placed to the right of the parent.
    Right,
}

impl PopoverSide {
    /// Returns the side that is the mirror image of this side.
    pub fn mirror(&self) -> Self {
        match self {
            PopoverSide::Top => PopoverSide::Bottom,
            PopoverSide::Bottom => PopoverSide::Top,
            PopoverSide::Left => PopoverSide::Right,
            PopoverSide::Right => PopoverSide::Left,
        }
    }
}

/// How the popover element should be aligned to the parent element. The alignment will be along an
/// axis that is perpendicular to the direction of the popover side. So for example, if the popup is
/// positioned below the parent, then the [`PopoverAlign`] variant controls the horizontal alignment
/// of the popup.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
pub enum PopoverAlign {
    /// The starting edge of the popover element should be aligned to the starting edge of the
    /// parent.
    #[default]
    Start,
    /// The center of the popover element should be aligned to the center of the parent.
    Center,
    /// The ending edge of the popover element should be aligned to the ending edge of the parent.
    End,
}

/// Indicates a possible position of a popover element relative to it's parent. You can
/// specify multiple possible positions; the positioning code will check to see if there is
/// sufficient space to display the popup without being clipped by the window edge. If any position
/// has sufficient room, it will pick the first one; if there are none, then it will pick the least
/// bad one.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
pub struct PopoverPlacement {
    /// The side of the parent entity where the popover element should be placed.
    pub side: PopoverSide,

    /// How the popover element should be aligned to the parent entity.
    pub align: PopoverAlign,

    /// The size of the gap between the parent and the popover element, in logical pixels. This will
    /// offset the popover along the direction of `side`.
    pub gap: f32,
}

/// Component which is inserted into a popover element to make it dynamically position relative to
/// a parent element.
#[derive(Component, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct Popover {
    /// List of potential positions for the popover element relative to the parent.
    pub positions: Vec<PopoverPlacement>,

    /// Indicates how close to the window edge the popup is allowed to go.
    pub window_margin: f32,
}

/// Shifts a [`Popover`] along its alignment axis to keep it inside its
/// collision boundary before trying another placement.
///
/// The inherited clip of the anchor is used as the collision boundary. Add
/// [`OverrideClip`] to the popover to use the entire render target instead.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct PopoverShift;

/// Hides a [`Popover`] after its anchor is fully clipped by an ancestor or the
/// render target.
///
/// While this component is present, [`PopoverPlugin`] controls the entity's
/// [`Visibility`].
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default, PartialEq)]
#[require(Visibility)]
pub struct PopoverHideWhenAnchorClipped;

/// A zero-sized, optional child of a [`Popover`] that is positioned at the
/// point where a callout arrow meets the popover edge.
///
/// [`Popover`] positions and rotates this entity after layout, in the same pass
/// as the popover itself. The arrow points up when its [`UiTransform`] has its
/// identity rotation. Visuals should be added as children so they inherit the
/// transform without affecting UI layout. Keeping the visual as a child also
/// lets headless users and themed UI layers provide different arrow shapes.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default, PartialEq)]
#[require(Node = popover_arrow_node(), PopoverDirection)]
pub struct PopoverArrow {
    /// Minimum distance between the arrow point and either corner of the
    /// popover, in logical pixels.
    pub corner_margin: f32,
}

fn popover_arrow_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Px(0.0),
        height: Val::Px(0.0),
        ..Default::default()
    }
}

impl Default for PopoverArrow {
    fn default() -> Self {
        Self { corner_margin: 0.0 }
    }
}

/// Direction written by [`PopoverPlugin`] for a [`PopoverArrow`].
///
/// Stylists use this to orient the arrow visual. It is output, not
/// configuration.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct PopoverDirection(pub PopoverSide);

impl Default for PopoverDirection {
    fn default() -> Self {
        Self(PopoverSide::Top)
    }
}

impl Clone for Popover {
    fn clone(&self) -> Self {
        Self {
            positions: self.positions.clone(),
            window_margin: self.window_margin,
        }
    }
}

pub(crate) fn position_popover(
    mut q_popover: Query<
        (
            Entity,
            &mut Node,
            &mut UiTransform,
            &mut UiGlobalTransform,
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &Popover,
            Has<OverrideClip>,
            Has<PopoverShift>,
            &ChildOf,
        ),
        Without<PopoverArrow>,
    >,
    mut qs_transform: ParamSet<(
        Query<
            (&ComputedNode, &UiGlobalTransform, Option<&CalculatedClip>),
            (Without<Popover>, Without<PopoverArrow>),
        >,
        Query<&mut UiGlobalTransform, (Without<Popover>, Without<PopoverArrow>)>,
    )>,
    mut q_arrows: Query<
        (
            &PopoverArrow,
            &ComputedNode,
            &mut UiTransform,
            &mut UiGlobalTransform,
            &mut PopoverDirection,
        ),
        (With<PopoverArrow>, Without<Popover>),
    >,
    q_children: Query<&Children>,
) {
    for (
        popover_entity,
        mut node,
        mut transform,
        mut ui_global_transform,
        computed_node,
        computed_target,
        popover,
        overrides_clip,
        shifts_into_view,
        parent,
    ) in q_popover.iter_mut()
    {
        let window_rect = Rect {
            min: Vec2::ZERO,
            max: computed_target.logical_size(),
        }
        .inflate(-popover.window_margin);

        // Compute the parent rectangle.
        let q_parent = qs_transform.p0();
        let Ok((parent_node, parent_transform, parent_clip)) = q_parent.get(parent.parent()) else {
            continue;
        };
        let collision_rect = if shifts_into_view {
            popover_boundary(
                computed_target.logical_size(),
                parent_clip,
                parent_node.inverse_scale_factor,
                overrides_clip,
                popover.window_margin,
            )
        } else {
            window_rect
        };

        // Computed node size includes the border, but since absolute positioning doesn't include
        // border we need to remove it from the calculations.
        let parent_size =
            parent_node.size() - parent_node.border.min_inset - parent_node.border.max_inset;
        let parent_rect = scale_rect(
            Rect::from_center_size(parent_transform.translation, parent_size),
            parent_node.inverse_scale_factor,
        );
        let parent_matrix = parent_transform.affine().matrix2;

        let mut best_occluded = f32::MAX;
        let mut best_rect = Rect::default();
        let mut best_placement = None;

        // Loop through all the potential positions and find a good one.
        for position in &popover.positions {
            let popover_size = computed_node.size() * computed_node.inverse_scale_factor;
            let mut rect = Rect::default();

            let target_width = popover_size.x;
            let target_height = popover_size.y;

            // Position along main axis.
            match position.side {
                PopoverSide::Top => {
                    rect.max.y = parent_rect.min.y - position.gap;
                    rect.min.y = rect.max.y - popover_size.y;
                }

                PopoverSide::Bottom => {
                    rect.min.y = parent_rect.max.y + position.gap;
                    rect.max.y = rect.min.y + popover_size.y;
                }

                PopoverSide::Left => {
                    rect.max.x = parent_rect.min.x - position.gap;
                    rect.min.x = rect.max.x - popover_size.x;
                }

                PopoverSide::Right => {
                    rect.min.x = parent_rect.max.x + position.gap;
                    rect.max.x = rect.min.x + popover_size.x;
                }
            }

            // Position along secondary axis.
            match position.align {
                PopoverAlign::Start => match position.side {
                    PopoverSide::Top | PopoverSide::Bottom => {
                        rect.min.x = parent_rect.min.x;
                        rect.max.x = rect.min.x + target_width;
                    }

                    PopoverSide::Left | PopoverSide::Right => {
                        rect.min.y = parent_rect.min.y;
                        rect.max.y = rect.min.y + target_height;
                    }
                },

                PopoverAlign::End => match position.side {
                    PopoverSide::Top | PopoverSide::Bottom => {
                        rect.max.x = parent_rect.max.x;
                        rect.min.x = rect.max.x - target_width;
                    }

                    PopoverSide::Left | PopoverSide::Right => {
                        rect.max.y = parent_rect.max.y;
                        rect.min.y = rect.max.y - target_height;
                    }
                },

                PopoverAlign::Center => match position.side {
                    PopoverSide::Top | PopoverSide::Bottom => {
                        rect.min.x = parent_rect.min.x + (parent_rect.width() - target_width) * 0.5;
                        rect.max.x = rect.min.x + target_width;
                    }

                    PopoverSide::Left | PopoverSide::Right => {
                        rect.min.y =
                            parent_rect.min.y + (parent_rect.height() - target_height) * 0.5;
                        rect.max.y = rect.min.y + target_height;
                    }
                },
            }

            if shifts_into_view {
                rect = shift_popover_rect(rect, collision_rect, position.side);
            }

            // Clip to window and see how much of the popover element is occluded. We can calculate
            // how much was clipped by intersecting the rectangle against the window bounds, and
            // then subtracting the area from the area of the unclipped rectangle.
            let clipped_rect = rect.intersect(collision_rect);
            let occlusion = rect.area() - clipped_rect.area();

            // Find the position that has the least occlusion.
            if occlusion < best_occluded {
                best_occluded = occlusion;
                best_rect = rect;
                best_placement = Some(*position);
            }
        }

        // Update node properties, but only if they are different from before (to avoid setting
        // change detection bit).
        if let Some(best_placement) = best_placement {
            let best_center = 0.5 * (best_rect.min + best_rect.max);
            let current_center =
                ui_global_transform.translation * computed_node.inverse_scale_factor;
            let physical_translation =
                (best_center - current_center) * computed_target.scale_factor();
            if parent_matrix.determinant() == 0.0 {
                continue;
            }
            let resolved_translation = transform.translation.resolve(
                computed_target.scale_factor(),
                computed_node.size(),
                computed_target.physical_size().as_vec2(),
            );
            let logical_translation = (resolved_translation
                + parent_matrix.inverse() * physical_translation)
                / computed_target.scale_factor();
            let ui_translation = Val2::px(logical_translation.x, logical_translation.y);
            if transform.translation != ui_translation {
                transform.translation = ui_translation;
            }
            if node.position_type != PositionType::Absolute {
                node.position_type = PositionType::Absolute;
            }

            if physical_translation != Vec2::ZERO {
                let mut affine = ui_global_transform.affine();
                affine.translation += physical_translation;
                *ui_global_transform = affine.into();

                if let Ok(children) = q_children.get(popover_entity) {
                    for child in children.iter() {
                        translate_ui_children_recursive(
                            *child,
                            physical_translation,
                            &q_children,
                            &mut qs_transform.p1(),
                        );
                    }
                }
            }

            position_popover_arrow(
                popover_entity,
                best_rect,
                parent_rect,
                best_placement,
                computed_target,
                &ui_global_transform,
                &q_children,
                &mut q_arrows,
                &mut qs_transform.p1(),
            );
        }
    }
}

fn position_popover_arrow(
    popover_entity: Entity,
    popover_rect: Rect,
    anchor_rect: Rect,
    placement: PopoverPlacement,
    computed_target: &ComputedUiRenderTargetInfo,
    popover_global_transform: &UiGlobalTransform,
    q_children: &Query<&Children>,
    q_arrows: &mut Query<
        (
            &PopoverArrow,
            &ComputedNode,
            &mut UiTransform,
            &mut UiGlobalTransform,
            &mut PopoverDirection,
        ),
        (With<PopoverArrow>, Without<Popover>),
    >,
    q_transform: &mut Query<&mut UiGlobalTransform, (Without<Popover>, Without<PopoverArrow>)>,
) {
    let Some(children) = q_children.get(popover_entity).ok() else {
        return;
    };
    let popover_matrix = popover_global_transform.affine().matrix2;
    if popover_matrix.determinant() == 0.0 {
        return;
    }

    for child in children.iter() {
        let Ok((arrow, arrow_node, mut transform, mut global_transform, mut direction)) =
            q_arrows.get_mut(*child)
        else {
            continue;
        };
        let (target, arrow_direction) =
            arrow_target(popover_rect, anchor_rect, placement, arrow.corner_margin);
        let rotation = arrow_rotation(arrow_direction);
        let current_physical = global_transform.translation;
        let target_physical = target * computed_target.scale_factor();
        let physical_translation = target_physical - current_physical;
        let resolved_translation = transform.translation.resolve(
            computed_target.scale_factor(),
            arrow_node.size(),
            computed_target.physical_size().as_vec2(),
        );
        let logical_translation = (resolved_translation
            + popover_matrix.inverse() * physical_translation)
            / computed_target.scale_factor();
        transform.translation = Val2::px(logical_translation.x, logical_translation.y);

        let rotation_delta = rotation * transform.rotation.inverse();
        transform.rotation = rotation;
        direction.set_if_neq(PopoverDirection(arrow_direction));

        let global_rotation =
            popover_matrix * Mat2::from(rotation_delta) * popover_matrix.inverse();
        let child_transform = Affine2::from_mat2_translation(
            global_rotation,
            target_physical - global_rotation * current_physical,
        );
        *global_transform = (child_transform * global_transform.affine()).into();

        if let Ok(children) = q_children.get(*child) {
            for child in children.iter() {
                transform_ui_children_recursive(*child, child_transform, q_children, q_transform);
            }
        }
    }
}

fn arrow_target(
    popover_rect: Rect,
    anchor_rect: Rect,
    placement: PopoverPlacement,
    corner_margin: f32,
) -> (Vec2, PopoverSide) {
    let margin = corner_margin.max(0.0);
    match placement.side {
        PopoverSide::Top => (
            Vec2::new(
                clamp_arrow_axis(
                    aligned_arrow_axis(
                        popover_rect.min.x,
                        popover_rect.max.x,
                        anchor_rect.min.x,
                        anchor_rect.max.x,
                        placement.align,
                        margin,
                    ),
                    popover_rect.min.x,
                    popover_rect.max.x,
                    margin,
                ),
                popover_rect.max.y,
            ),
            PopoverSide::Bottom,
        ),
        PopoverSide::Bottom => (
            Vec2::new(
                clamp_arrow_axis(
                    aligned_arrow_axis(
                        popover_rect.min.x,
                        popover_rect.max.x,
                        anchor_rect.min.x,
                        anchor_rect.max.x,
                        placement.align,
                        margin,
                    ),
                    popover_rect.min.x,
                    popover_rect.max.x,
                    margin,
                ),
                popover_rect.min.y,
            ),
            PopoverSide::Top,
        ),
        PopoverSide::Left => (
            Vec2::new(
                popover_rect.max.x,
                clamp_arrow_axis(
                    aligned_arrow_axis(
                        popover_rect.min.y,
                        popover_rect.max.y,
                        anchor_rect.min.y,
                        anchor_rect.max.y,
                        placement.align,
                        margin,
                    ),
                    popover_rect.min.y,
                    popover_rect.max.y,
                    margin,
                ),
            ),
            PopoverSide::Right,
        ),
        PopoverSide::Right => (
            Vec2::new(
                popover_rect.min.x,
                clamp_arrow_axis(
                    aligned_arrow_axis(
                        popover_rect.min.y,
                        popover_rect.max.y,
                        anchor_rect.min.y,
                        anchor_rect.max.y,
                        placement.align,
                        margin,
                    ),
                    popover_rect.min.y,
                    popover_rect.max.y,
                    margin,
                ),
            ),
            PopoverSide::Left,
        ),
    }
}

fn aligned_arrow_axis(
    popover_min: f32,
    popover_max: f32,
    anchor_min: f32,
    anchor_max: f32,
    align: PopoverAlign,
    margin: f32,
) -> f32 {
    let position = match align {
        PopoverAlign::Start => 1.0 / 3.0,
        PopoverAlign::Center => 0.5,
        PopoverAlign::End => 2.0 / 3.0,
    };
    let popover_axis = popover_min + (popover_max - popover_min) * position;
    if anchor_max - anchor_min <= margin * 2.0 {
        (anchor_min + anchor_max) * 0.5
    } else {
        popover_axis.clamp(anchor_min + margin, anchor_max - margin)
    }
}

fn clamp_arrow_axis(value: f32, min: f32, max: f32, margin: f32) -> f32 {
    let available = max - min;
    if available <= margin * 2.0 {
        (min + max) * 0.5
    } else {
        value.clamp(min + margin, max - margin)
    }
}

fn arrow_rotation(direction: PopoverSide) -> Rot2 {
    match direction {
        PopoverSide::Top => Rot2::IDENTITY,
        PopoverSide::Right => Rot2::FRAC_PI_2,
        PopoverSide::Bottom => Rot2::PI,
        PopoverSide::Left => Rot2::FRAC_PI_2.inverse(),
    }
}

fn transform_ui_children_recursive(
    entity: Entity,
    transform: Affine2,
    q_children: &Query<&Children>,
    q_transform: &mut Query<&mut UiGlobalTransform, (Without<Popover>, Without<PopoverArrow>)>,
) {
    let Ok(mut ui_global_transform) = q_transform.get_mut(entity) else {
        return;
    };

    *ui_global_transform = (transform * ui_global_transform.affine()).into();

    if let Ok(children) = q_children.get(entity) {
        for child in children.iter() {
            transform_ui_children_recursive(*child, transform, q_children, q_transform);
        }
    }
}

fn translate_ui_children_recursive(
    entity: Entity,
    translation: Vec2,
    q_children: &Query<&Children>,
    q_transform: &mut Query<&mut UiGlobalTransform, (Without<Popover>, Without<PopoverArrow>)>,
) {
    let Ok(mut ui_global_transform) = q_transform.get_mut(entity) else {
        return;
    };

    *ui_global_transform = translate_global(ui_global_transform.affine(), translation).into();

    if let Ok(children) = q_children.get(entity) {
        for child in children.iter() {
            translate_ui_children_recursive(*child, translation, q_children, q_transform);
        }
    }
}

fn translate_global(transform: Affine2, translation: Vec2) -> Affine2 {
    Affine2::from_translation(translation) * transform
}

fn update_popover_anchor_visibility(
    mut popovers: Query<
        (&ChildOf, &ComputedUiRenderTargetInfo, &mut Visibility),
        (With<Popover>, With<PopoverHideWhenAnchorClipped>),
    >,
    anchors: Query<(&ComputedNode, &UiGlobalTransform, Option<&CalculatedClip>)>,
) {
    for (parent, computed_target, mut visibility) in &mut popovers {
        let reference_hidden = anchors
            .get(parent.parent())
            .map(|(anchor_node, anchor_transform, inherited_clip)| {
                let anchor_size = anchor_node.size()
                    - anchor_node.border.min_inset
                    - anchor_node.border.max_inset;
                let anchor_rect = scale_rect(
                    Rect::from_center_size(anchor_transform.translation, anchor_size),
                    anchor_node.inverse_scale_factor,
                );
                reference_is_fully_clipped(
                    anchor_rect,
                    inherited_clip,
                    computed_target.logical_size(),
                    anchor_node.inverse_scale_factor,
                )
            })
            .unwrap_or(true);
        visibility.set_if_neq(if reference_hidden {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
    }
}

/// Plugin that adds systems for the [`Popover`] component.
pub struct PopoverPlugin;

impl Plugin for PopoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            position_popover
                .in_set(UiSystems::Layout)
                .after(ui_layout_system)
                .before(update_scrollbar_thumb),
        )
        .add_systems(
            PostUpdate,
            update_popover_anchor_visibility
                .after(UiSystems::PostLayout)
                .before(bevy_app::TransformGizmoRenderStep),
        )
        .world_mut()
        .register_component_hooks::<PopoverHideWhenAnchorClipped>()
        .on_remove(|mut world, context| {
            if let Some(mut visibility) = world.get_mut::<Visibility>(context.entity) {
                *visibility = Visibility::Inherited;
            }
        });
    }
}

#[inline]
fn scale_rect(rect: Rect, factor: f32) -> Rect {
    Rect {
        min: rect.min * factor,
        max: rect.max * factor,
    }
}

fn popover_boundary(
    target_size: Vec2,
    inherited_clip: Option<&CalculatedClip>,
    inverse_scale_factor: f32,
    overrides_clip: bool,
    margin: f32,
) -> Rect {
    let target_rect = Rect::from_corners(Vec2::ZERO, target_size);
    if overrides_clip {
        target_rect
    } else {
        inherited_clip.map_or(target_rect, |clip| {
            target_rect.intersect(scale_rect(clip.clip, inverse_scale_factor))
        })
    }
    .inflate(-margin)
}

fn shift_popover_rect(mut rect: Rect, boundary: Rect, side: PopoverSide) -> Rect {
    let delta = match side {
        PopoverSide::Top | PopoverSide::Bottom => {
            shift_axis(rect.min.x, rect.max.x, boundary.min.x, boundary.max.x)
        }
        PopoverSide::Left | PopoverSide::Right => {
            shift_axis(rect.min.y, rect.max.y, boundary.min.y, boundary.max.y)
        }
    };
    match side {
        PopoverSide::Top | PopoverSide::Bottom => {
            rect.min.x += delta;
            rect.max.x += delta;
        }
        PopoverSide::Left | PopoverSide::Right => {
            rect.min.y += delta;
            rect.max.y += delta;
        }
    }
    rect
}

fn shift_axis(min: f32, max: f32, boundary_min: f32, boundary_max: f32) -> f32 {
    if max - min > boundary_max - boundary_min {
        return 0.5 * (boundary_min + boundary_max - min - max);
    }
    if min < boundary_min {
        boundary_min - min
    } else if max > boundary_max {
        boundary_max - max
    } else {
        0.0
    }
}

fn reference_is_fully_clipped(
    reference_rect: Rect,
    inherited_clip: Option<&CalculatedClip>,
    target_size: Vec2,
    inverse_scale_factor: f32,
) -> bool {
    let target_rect = Rect::from_corners(Vec2::ZERO, target_size);
    let visible_rect = reference_rect.intersect(target_rect);
    let visible_rect = inherited_clip.map_or(visible_rect, |clip| {
        visible_rect.intersect(scale_rect(clip.clip, inverse_scale_factor))
    });
    visible_rect.area() <= 0.0
}
