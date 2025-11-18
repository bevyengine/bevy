use crate::ui_surface::UiSurface;
use crate::widget::{ImageMeasure, ImageNode, ImageNodeSize, NodeImageMode};
use crate::{
    experimental::UiRootNodes, DefaultUiCamera, UiContainerTarget, UiScale, UiSystems,
    UiTargetCamera,
};
use crate::{
    AmbiguousWithText, AmbiguousWithUpdateText2dLayout, ContentSize, FocusPolicy, IgnoreScroll,
    Interaction, LayoutConfig, LayoutContext, NodeMeasure, NodeQuery, NodeQueryItem, Outline,
    RelativeCursorPosition, ScrollPosition, State, UiContainerSize, UiStack, UiTransform,
};
use bevy_asset::Assets;
use bevy_ecs::entity::ContainsEntity;
use bevy_image::{Image, TextureAtlasLayout, TRANSPARENT_IMAGE_HANDLE};
use bevy_transform::TransformSystems;

use crate::{
    experimental::UiChildren, ui_transform::UiGlobalTransform, ComputedUiRenderTargetInfo,
    ComputedUiTargetCamera, Display, Node, OverflowAxis, OverrideClip,
};

use super::ComputedNode;
use bevy_app::{HierarchyPropagatePlugin, Plugin, PostUpdate, PreUpdate, Propagate, PropagateSet};
use bevy_camera::{Camera, NormalizedRenderTarget};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::lifecycle::RemovedComponents;
use bevy_ecs::query::{Added, Or, With, Without};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Local, ResMut};
use bevy_ecs::world::Ref;
use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query, Res},
};
use bevy_input::mouse::MouseButton;
use bevy_input::touch::Touches;
use bevy_input::{ButtonInput, InputSystems};
use bevy_math::{Affine2, Vec2, Vec3Swizzles};
use bevy_platform::collections::HashMap;
use bevy_sprite::Anchor;
use bevy_sprite::BorderRect;
use bevy_text::{ComputedTextBlock, CosmicFontSystem};
use bevy_transform::components::GlobalTransform;
use bevy_window::{PrimaryWindow, Window};

#[derive(Default)]
pub struct UiContainerPlugin;

impl Plugin for UiContainerPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_sets(
            PostUpdate,
            PropagateSet::<UiContainerTarget>::default().in_set(UiSystems::Propagate),
        )
        .add_plugins(HierarchyPropagatePlugin::<UiContainerTarget>::new(
            PostUpdate,
        ))
        .add_systems(
            PreUpdate,
            ui_focus_system.in_set(UiSystems::Focus).after(InputSystems),
        );

        let ui_layout_system_config = ui_layout_system
            .in_set(UiSystems::Layout)
            .before(TransformSystems::Propagate);

        let ui_layout_system_config = ui_layout_system_config
            // Text and Text2D operate on disjoint sets of entities
            .ambiguous_with(bevy_sprite::update_text2d_layout)
            .ambiguous_with(bevy_text::detect_text_needs_rerender::<bevy_sprite::Text2d>);

        app.add_systems(
            PostUpdate,
            (
                (
                    propagate_ui_target_cameras,
                    // ui_layout_change_update
                )
                    .in_set(UiSystems::Prepare),
                ui_layout_system_config,
                // Potential conflicts: `Assets<Image>`
                // They run independently since `widget::image_node_system` will only ever observe
                // its own ImageNode, and `widget::text_system` & `bevy_text::update_text2d_layout`
                // will never modify a pre-existing `Image` asset.
                update_image_content_size_system
                    .in_set(UiSystems::Content)
                    .in_set(AmbiguousWithText)
                    .in_set(AmbiguousWithUpdateText2dLayout),
            ),
        );
    }
}

pub fn ui_layout_change_update(
    mut ui_surface: ResMut<UiSurface>,
    container_target_add: Query<Entity, Added<UiContainerTarget>>,

    ui_surface_query: Query<&mut UiSurface>,
    mut removed_container_target: RemovedComponents<UiContainerTarget>,
) {
    if !container_target_add.is_empty() {
        ui_surface.remove_entities(container_target_add.iter());
    }

    if !removed_container_target.is_empty() {
        let collect = removed_container_target.read().collect::<Vec<_>>();
        for mut ui_surface in ui_surface_query {
            ui_surface.remove_entities_ref(collect.iter());
        }
    }
}

pub fn propagate_ui_target_cameras(
    mut commands: Commands,
    default_ui_camera: DefaultUiCamera,
    target_camera_query: Query<&UiTargetCamera>,
    ui_root_nodes: UiRootNodes<With<UiContainerTarget>>,
    query_ui_scale: Query<(&UiScale, &UiContainerSize)>,
    query_target: Query<&UiContainerTarget>,
) {
    let default_camera_entity = default_ui_camera.get();

    for root_entity in ui_root_nodes.iter() {
        let Ok(target) = query_target.get(root_entity) else {
            continue;
        };

        let camera = target_camera_query
            .get(root_entity)
            .ok()
            .map(UiTargetCamera::entity)
            .or(default_camera_entity)
            .unwrap_or(Entity::PLACEHOLDER);

        commands
            .entity(root_entity)
            .try_insert(Propagate(ComputedUiTargetCamera { camera }));

        let Ok((scale, size)) = query_ui_scale.get(target.0) else {
            return;
        };

        commands
            .entity(root_entity)
            .try_insert(Propagate(ComputedUiRenderTargetInfo {
                scale_factor: scale.0,
                physical_size: size.0,
            }));
    }
}

/// Updates the UI's layout tree, computes the new layout geometry and then updates the sizes and transforms of all the UI nodes.
pub fn ui_layout_system(
    ui_root_node_query: UiRootNodes<With<UiContainerTarget>>,
    ui_children: UiChildren<With<UiContainerTarget>>,
    mut node_query: Query<(
        Entity,
        Ref<Node>,
        Option<&mut ContentSize>,
        Ref<ComputedUiRenderTargetInfo>,
        Ref<UiContainerTarget>,
    )>,
    added_node_query: Query<(), Or<(Added<Node>, With<UiContainerTarget>)>>,
    mut node_update_query: Query<(
        &mut ComputedNode,
        &UiTransform,
        &mut UiGlobalTransform,
        &Node,
        Option<&LayoutConfig>,
        Option<&Outline>,
        Option<&ScrollPosition>,
        Option<&IgnoreScroll>,
        &UiContainerTarget,
    )>,
    mut buffer_query: Query<&mut ComputedTextBlock>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut removed_children: RemovedComponents<Children>,
    mut removed_content_sizes: RemovedComponents<ContentSize>,
    mut removed_nodes: RemovedComponents<Node>,
    mut ui_surface_query: Query<&mut UiSurface>,
    contain_query: Query<(&GlobalTransform, &UiContainerSize, &Anchor)>,
) {
    // When a `ContentSize` component is removed from an entity, we need to remove the measure from the corresponding taffy node.
    for entity in removed_content_sizes.read() {
        for mut ui_surface in &mut ui_surface_query {
            ui_surface.try_remove_node_context(entity);
        }
    }

    // Sync Node and ContentSize to Taffy for all nodes
    node_query.iter_mut().for_each(
        |(entity, node, content_size, computed_target, container_target)| {
            if computed_target.is_changed()
                || node.is_changed()
                || content_size
                    .as_ref()
                    .is_some_and(|c| c.is_changed() || c.measure.is_some())
                || container_target.is_changed()
            {
                let layout_context = LayoutContext::new(
                    computed_target.scale_factor,
                    computed_target.physical_size.as_vec2(),
                );
                let measure = content_size.and_then(|mut c| c.measure.take());

                let Ok(mut ui_surface) = ui_surface_query.get_mut(container_target.0) else {
                    tracing::error!(
                        "Node {:?} storage error, UI container {:?} is invalid",
                        entity,
                        container_target.0
                    );
                    return;
                };
                ui_surface.upsert_node(&layout_context, entity, &node, measure);
            }
        },
    );

    // update and remove children
    for entity in removed_children.read() {
        for mut ui_surface in &mut ui_surface_query {
            ui_surface.try_remove_children(entity);
        }
    }

    let removed_nodes = removed_nodes
        .read()
        .filter(|entity| !node_query.contains(*entity))
        .collect::<Vec<_>>();

    for mut ui_surface in &mut ui_surface_query {
        ui_surface.remove_entities_ref(removed_nodes.iter());
    }

    for ui_root_entity in ui_root_node_query.iter() {
        fn update_children_recursively(
            ui_surface: &mut UiSurface,
            ui_children: &UiChildren<With<UiContainerTarget>>,
            added_node_query: &Query<(), Or<(Added<Node>, With<UiContainerTarget>)>>,
            entity: Entity,
        ) {
            if ui_surface.entity_to_taffy.contains_key(&entity)
                && (added_node_query.contains(entity)
                    || ui_children.is_changed(entity)
                    || ui_children
                        .iter_ui_children(entity)
                        .any(|child| added_node_query.contains(child)))
            {
                ui_surface.update_children(entity, ui_children.iter_ui_children(entity));
            }

            for child in ui_children.iter_ui_children(entity) {
                update_children_recursively(ui_surface, ui_children, added_node_query, child);
            }
        }

        let Ok((.., computed_target, container_target)) = node_query.get(ui_root_entity) else {
            continue;
        };

        let Ok(mut ui_surface) = ui_surface_query.get_mut(container_target.0) else {
            continue;
        };

        update_children_recursively(
            &mut ui_surface,
            &ui_children,
            &added_node_query,
            ui_root_entity,
        );

        ui_surface.compute_layout(
            ui_root_entity,
            computed_target.physical_size,
            &mut buffer_query,
            &mut font_system,
        );

        update_uinode_geometry_recursive(
            ui_root_entity,
            &mut ui_surface,
            true,
            computed_target.physical_size().as_vec2(),
            Affine2::IDENTITY,
            &mut node_update_query,
            &ui_children,
            computed_target.scale_factor.recip(),
            Vec2::ZERO,
            Vec2::ZERO,
            &contain_query,
        );
    }

    // Returns the combined bounding box of the node and any of its overflowing children.
    fn update_uinode_geometry_recursive(
        entity: Entity,
        ui_surface: &mut UiSurface,
        inherited_use_rounding: bool,
        target_size: Vec2,
        mut inherited_transform: Affine2,
        node_update_query: &mut Query<(
            &mut ComputedNode,
            &UiTransform,
            &mut UiGlobalTransform,
            &Node,
            Option<&LayoutConfig>,
            Option<&Outline>,
            Option<&ScrollPosition>,
            Option<&IgnoreScroll>,
            &UiContainerTarget,
        )>,
        ui_children: &UiChildren<With<UiContainerTarget>>,
        inverse_target_scale_factor: f32,
        parent_size: Vec2,
        parent_scroll_position: Vec2,
        contain_query: &Query<(&GlobalTransform, &UiContainerSize, &Anchor)>,
    ) {
        // Transform the node coordinate system
        let flip_y = Affine2::from_scale(Vec2::new(1.0, -1.0));

        if let Ok((
            mut node,
            transform,
            mut global_transform,
            style,
            maybe_layout_config,
            maybe_outline,
            maybe_scroll_position,
            maybe_scroll_sticky,
            container_target,
        )) = node_update_query.get_mut(entity)
        {
            let use_rounding = maybe_layout_config
                .map(|layout_config| layout_config.use_rounding)
                .unwrap_or(inherited_use_rounding);

            let Ok((layout, unrounded_size)) = ui_surface.get_layout(entity, use_rounding) else {
                return;
            };

            let layout_size = Vec2::new(layout.size.width, layout.size.height);

            // Taffy layout position of the top-left corner of the node, relative to its parent.
            let layout_location = Vec2::new(layout.location.x, layout.location.y);

            // If IgnoreScroll is set, parent scroll position is ignored along the specified axes.
            let effective_parent_scroll = maybe_scroll_sticky
                .map(|scroll_sticky| parent_scroll_position * Vec2::from(!scroll_sticky.0))
                .unwrap_or(parent_scroll_position);

            // The position of the center of the node relative to its top-left corner.
            let local_center =
                layout_location - effective_parent_scroll + 0.5 * (layout_size - parent_size);

            // only trigger change detection when the new values are different
            if node.size != layout_size
                || node.unrounded_size != unrounded_size
                || node.inverse_scale_factor != inverse_target_scale_factor
            {
                node.size = layout_size;
                node.unrounded_size = unrounded_size;
                node.inverse_scale_factor = inverse_target_scale_factor;
            }

            let content_size = Vec2::new(layout.content_size.width, layout.content_size.height);
            node.bypass_change_detection().content_size = content_size;

            let taffy_rect_to_border_rect = |rect: taffy::Rect<f32>| BorderRect {
                left: rect.left,
                right: rect.right,
                top: rect.top,
                bottom: rect.bottom,
            };

            node.bypass_change_detection().border = taffy_rect_to_border_rect(layout.border);
            node.bypass_change_detection().padding = taffy_rect_to_border_rect(layout.padding);

            // Compute the node's new global transform
            let mut local_transform = transform.compute_affine(
                inverse_target_scale_factor.recip(),
                layout_size,
                target_size,
            );

            // Coordinate correction for root node
            if ui_children.get_parent(entity).is_none()
                && let Ok((global, contain, anchor)) = contain_query.get(container_target.0)
            {
                local_transform.translation += global.translation().xy();

                // Root node center offset
                let offset = flip_y.transform_vector2(contain.0.as_vec2());
                local_transform.translation -= offset / 2.0;

                // Anchor offset
                let offset_anchor = anchor.as_vec() * contain.0.as_vec2();
                local_transform.translation -= offset_anchor;
            }

            local_transform.translation += flip_y.transform_vector2(local_center);
            inherited_transform *= local_transform;

            if inherited_transform != **global_transform {
                *global_transform = inherited_transform.into();
            }

            // We don't trigger change detection for changes to border radius
            node.bypass_change_detection().border_radius = style.border_radius.resolve(
                inverse_target_scale_factor.recip(),
                node.size,
                target_size,
            );

            if let Some(outline) = maybe_outline {
                // don't trigger change detection when only outlines are changed
                let node = node.bypass_change_detection();
                node.outline_width = if style.display != Display::None {
                    outline
                        .width
                        .resolve(
                            inverse_target_scale_factor.recip(),
                            node.size().x,
                            target_size,
                        )
                        .unwrap_or(0.)
                        .max(0.)
                } else {
                    0.
                };

                node.outline_offset = outline
                    .offset
                    .resolve(
                        inverse_target_scale_factor.recip(),
                        node.size().x,
                        target_size,
                    )
                    .unwrap_or(0.)
                    .max(0.);
            }

            node.bypass_change_detection().scrollbar_size =
                Vec2::new(layout.scrollbar_size.width, layout.scrollbar_size.height);

            let scroll_position: Vec2 = maybe_scroll_position
                .map(|scroll_pos| {
                    Vec2::new(
                        if style.overflow.x == OverflowAxis::Scroll {
                            scroll_pos.x * inverse_target_scale_factor.recip()
                        } else {
                            0.0
                        },
                        if style.overflow.y == OverflowAxis::Scroll {
                            scroll_pos.y * inverse_target_scale_factor.recip()
                        } else {
                            0.0
                        },
                    )
                })
                .unwrap_or_default();

            let max_possible_offset =
                (content_size - layout_size + node.scrollbar_size).max(Vec2::ZERO);
            let clamped_scroll_position = scroll_position.clamp(Vec2::ZERO, max_possible_offset);

            let physical_scroll_position = clamped_scroll_position.floor();

            node.bypass_change_detection().scroll_position = physical_scroll_position;

            for child_uinode in ui_children.iter_ui_children(entity) {
                update_uinode_geometry_recursive(
                    child_uinode,
                    ui_surface,
                    use_rounding,
                    target_size,
                    inherited_transform,
                    node_update_query,
                    ui_children,
                    inverse_target_scale_factor,
                    layout_size,
                    physical_scroll_position,
                    contain_query,
                );
            }
        }
    }
}

/// The system that sets Interaction for all UI elements based on the mouse cursor activity
///
/// Entities with a hidden [`InheritedVisibility`] are always treated as released.
pub fn ui_focus_system(
    mut hovered_nodes: Local<Vec<Entity>>,
    mut state: Local<State>,
    camera_query: Query<(Entity, &Camera)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<&Window>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    touches_input: Res<Touches>,
    ui_stack: Res<UiStack>,
    mut node_query: Query<NodeQuery, With<UiContainerTarget>>,
    clipping_query: Query<(&ComputedNode, &UiGlobalTransform, &Node), With<UiContainerTarget>>,
    child_of_query: Query<&ChildOf, Without<OverrideClip>>,
    global_transform_query: Query<&GlobalTransform>,
) {
    let primary_window = primary_window.iter().next();

    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(NodeQueryItem {
            interaction: Some(mut interaction),
            ..
        }) = node_query.get_mut(entity)
        {
            *interaction = Interaction::None;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();
    if mouse_released {
        for node in &mut node_query {
            if let Some(mut interaction) = node.interaction
                && *interaction == Interaction::Pressed
            {
                *interaction = Interaction::None;
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.any_just_pressed();

    let camera_cursor_positions: HashMap<Entity, Vec2> = camera_query
        .iter()
        .filter_map(|(entity, camera)| {
            let Ok(position) = global_transform_query.get(entity) else {
                return None;
            };

            // Interactions are only supported for cameras rendering to a window.
            let Some(NormalizedRenderTarget::Window(window_ref)) =
                camera.target.normalize(primary_window)
            else {
                return None;
            };
            let window = windows.get(window_ref.entity()).ok()?;

            window
                .cursor_position()
                .map(|cursor| camera.viewport_to_world(position, cursor))
                .map(|ray| ray.unwrap().origin.truncate())
                .map(|world_position| (entity, world_position))
        })
        .collect();

    // prepare an iterator that contains all the nodes that have the cursor in their rect,
    // from the top node to the bottom one. this will also reset the interaction to `None`
    // for all nodes encountered that are no longer hovered.

    hovered_nodes.clear();
    // reverse the iterator to traverse the tree from closest slice to furthest
    for uinodes in ui_stack
        .partition
        .iter()
        .rev()
        .map(|range| &ui_stack.uinodes[range.clone()])
    {
        // Retrieve the first node and resolve its camera target.
        // Only need to do this once per slice, as all the nodes in the slice share the same camera.
        let Ok(root_node) = node_query.get_mut(uinodes[0]) else {
            continue;
        };

        let Some(camera_entity) = root_node.target_camera.get() else {
            continue;
        };

        let cursor_position = camera_cursor_positions.get(&camera_entity);

        for entity in uinodes.iter().rev().cloned() {
            let Ok(node) = node_query.get_mut(entity) else {
                continue;
            };

            let Some(inherited_visibility) = node.inherited_visibility else {
                continue;
            };

            // Nodes that are not rendered should not be interactable
            if !inherited_visibility.get() {
                // Reset their interaction to None to avoid strange stuck state
                if let Some(mut interaction) = node.interaction {
                    // We cannot simply set the interaction to None, as that will trigger change detection repeatedly
                    interaction.set_if_neq(Interaction::None);
                }
                continue;
            }

            let contains_cursor = cursor_position.is_some_and(|point| {
                node.node.contains_point(*node.transform, *point)
                    && clip_check_recursive(*point, entity, &clipping_query, &child_of_query)
            });

            // The mouse position relative to the node
            // (-0.5, -0.5) is the top-left corner, (0.5, 0.5) is the bottom-right corner
            // Coordinates are relative to the entire node, not just the visible region.
            let normalized_cursor_position = cursor_position.and_then(|cursor_position| {
                // ensure node size is non-zero in all dimensions, otherwise relative position will be
                // +/-inf. if the node is hidden, the visible rect min/max will also be -inf leading to
                // false positives for mouse_over (#12395)
                node.node.normalize_point(*node.transform, *cursor_position)
            });

            // If the current cursor position is within the bounds of the node's visible area, consider it for
            // clicking
            let relative_cursor_position_component = RelativeCursorPosition {
                cursor_over: contains_cursor,
                normalized: normalized_cursor_position,
            };

            // Save the relative cursor position to the correct component
            if let Some(mut node_relative_cursor_position_component) = node.relative_cursor_position
            {
                // Avoid triggering change detection when not necessary.
                node_relative_cursor_position_component
                    .set_if_neq(relative_cursor_position_component);
            }

            if contains_cursor {
                hovered_nodes.push(entity);
            } else {
                if let Some(mut interaction) = node.interaction
                    && (*interaction == Interaction::Hovered
                        || (normalized_cursor_position.is_none()))
                {
                    interaction.set_if_neq(Interaction::None);
                }
                continue;
            }
        }
    }

    // set Pressed or Hovered on top nodes. as soon as a node with a `Block` focus policy is detected,
    // the iteration will stop on it because it "captures" the interaction.
    let mut hovered_nodes = hovered_nodes.iter();
    let mut iter = node_query.iter_many_mut(hovered_nodes.by_ref());
    while let Some(node) = iter.fetch_next() {
        if let Some(mut interaction) = node.interaction {
            if mouse_clicked {
                // only consider nodes with Interaction "pressed"
                if *interaction != Interaction::Pressed {
                    *interaction = Interaction::Pressed;
                    // if the mouse was simultaneously released, reset this Interaction in the next
                    // frame
                    if mouse_released {
                        state.entities_to_reset.push(node.entity);
                    }
                }
            } else if *interaction == Interaction::None {
                *interaction = Interaction::Hovered;
            }
        }

        match node.focus_policy.unwrap_or(&FocusPolicy::Block) {
            FocusPolicy::Block => {
                break;
            }
            FocusPolicy::Pass => { /* allow the next node to be hovered/pressed */ }
        }
    }
    // reset `Interaction` for the remaining lower nodes to `None`. those are the nodes that remain in
    // `moused_over_nodes` after the previous loop is exited.
    let mut iter = node_query.iter_many_mut(hovered_nodes);
    while let Some(node) = iter.fetch_next() {
        if let Some(mut interaction) = node.interaction {
            // don't reset pressed nodes because they're handled separately
            if *interaction != Interaction::Pressed {
                interaction.set_if_neq(Interaction::None);
            }
        }
    }
}

/// Walk up the tree child-to-parent checking that `point` is not clipped by any ancestor node.
/// If `entity` has an [`OverrideClip`] component it ignores any inherited clipping and returns true.
pub fn clip_check_recursive(
    point: Vec2,
    entity: Entity,
    clipping_query: &Query<
        '_,
        '_,
        (&ComputedNode, &UiGlobalTransform, &Node),
        With<UiContainerTarget>,
    >,
    child_of_query: &Query<&ChildOf, Without<OverrideClip>>,
) -> bool {
    if let Ok(child_of) = child_of_query.get(entity) {
        let parent = child_of.0;
        if let Ok((computed_node, transform, node)) = clipping_query.get(parent)
            && !computed_node
                .resolve_clip_rect(node.overflow, node.overflow_clip_margin)
                .contains(transform.inverse().transform_point2(point))
        {
            // The point is clipped and should be ignored by picking
            return false;
        }
        return clip_check_recursive(point, parent, clipping_query, child_of_query);
    }
    // Reached root, point unclipped by all ancestors
    true
}

type UpdateImageFilter = (With<Node>, Without<crate::prelude::Text>);

/// Updates content size of the node based on the image provided
pub fn update_image_content_size_system(
    textures: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    mut query: Query<
        (
            &mut ContentSize,
            Ref<ImageNode>,
            &mut ImageNodeSize,
            Ref<ComputedUiRenderTargetInfo>,
            Ref<UiContainerTarget>,
        ),
        UpdateImageFilter,
    >,
) {
    for (mut content_size, image, mut image_size, computed_target, ui_container) in &mut query {
        if !matches!(image.image_mode, NodeImageMode::Auto)
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
        {
            if image.is_changed() {
                // Mutably derefs, marking the `ContentSize` as changed ensuring `ui_layout_system` will remove the node's measure func if present.
                content_size.measure = None;
            }
            continue;
        }

        if let Some(size) =
            image
                .rect
                .map(|rect| rect.size().as_uvec2())
                .or_else(|| match &image.texture_atlas {
                    Some(atlas) => atlas.texture_rect(&atlases).map(|t| t.size()),
                    None => textures.get(&image.image).map(Image::size),
                })
        {
            // Update only if size or scale factor has changed to avoid needless layout calculations
            if size != image_size.size
                || computed_target.is_changed()
                || content_size.is_added()
                || ui_container.is_changed()
            {
                image_size.size = size;
                content_size.set(NodeMeasure::Image(ImageMeasure {
                    // multiply the image size by the scale factor to get the physical size
                    size: size.as_vec2() * computed_target.scale_factor(),
                }));
            }
        }
    }
}
