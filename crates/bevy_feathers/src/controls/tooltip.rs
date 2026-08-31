//! Feathers rendering for the headless tooltip widget.

use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PostUpdate};
use bevy_color::{Alpha, Srgba};
use bevy_ecs::{entity::EntityHashMap, hierarchy::ChildOf, prelude::*};
use bevy_log::warn;
use bevy_math::Rot2;
use bevy_picking::Pickable;
use bevy_scene::prelude::*;
use bevy_text::{FontSourceTemplate, FontWeight, TextFont};
use bevy_ui::{
    px, widget::Text, BoxShadow, ComputedNode, GlobalZIndex, Node, Overflow, OverrideClip,
    PositionType, UiRect, UiSystems, UiTransform,
};
use bevy_ui_widgets::{
    popover::{Popover, PopoverPlacement, PopoverPlugin, PopoverSide, ResolvedPopoverPlacement},
    HideTooltip, ShowTooltip, Tooltip, TooltipContent, TooltipDetail, TooltipPlugin, TooltipPopup,
};

use crate::{
    constants::{fonts, size},
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor},
    tokens,
};

const TOOLTIP_ARROW_SIZE: f32 = 10.0;
const TOOLTIP_ARROW_EDGE_MARGIN: f32 = 4.0;

#[derive(Component, Default, Clone)]
struct FeathersTooltipRoot {
    arrow: bool,
}

#[derive(Component, Default, Clone)]
struct FeathersTooltipArrowClip;

#[derive(Component, Default, Clone)]
struct FeathersTooltipArrow;

#[derive(Resource, Default)]
struct TooltipRoots(EntityHashMap<Entity>);

fn placement_candidates(preferred: PopoverPlacement) -> [PopoverPlacement; 4] {
    let sides = match preferred.side {
        PopoverSide::Top => [
            PopoverSide::Top,
            PopoverSide::Bottom,
            PopoverSide::Right,
            PopoverSide::Left,
        ],
        PopoverSide::Bottom => [
            PopoverSide::Bottom,
            PopoverSide::Top,
            PopoverSide::Right,
            PopoverSide::Left,
        ],
        PopoverSide::Left => [
            PopoverSide::Left,
            PopoverSide::Right,
            PopoverSide::Top,
            PopoverSide::Bottom,
        ],
        PopoverSide::Right => [
            PopoverSide::Right,
            PopoverSide::Left,
            PopoverSide::Top,
            PopoverSide::Bottom,
        ],
    };
    sides.map(|side| PopoverPlacement { side, ..preferred })
}

fn tooltip_scene(text: String, placement: PopoverPlacement, arrow: bool) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(6), px(4)),
            border: px(1),
            border_radius: px(4),
        }
        FeathersTooltipRoot {
            arrow: { arrow },
        }
        Pickable::IGNORE
        GlobalZIndex(101)
        OverrideClip
        ThemeBackgroundColor(tokens::TOOLTIP_BG)
        ThemeBorderColor(tokens::TOOLTIP_BORDER)
        BoxShadow::new(
            Srgba::BLACK.with_alpha(0.9).into(),
            px(0),
            px(0),
            px(1),
            px(4),
        )
        Popover {
            positions: { placement_candidates(placement).to_vec() },
            window_margin: 4.0,
        }
        Children [
            (
                Text(text)
                TextFont {
                    font: FontSourceTemplate::Handle(fonts::REGULAR),
                    font_size: size::COMPACT_FONT,
                    weight: FontWeight::NORMAL,
                }
                ThemeTextColor(tokens::TOOLTIP_TEXT)
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    width: px(0),
                    height: px(0),
                    overflow: Overflow::clip(),
                }
                FeathersTooltipArrowClip
                Pickable::IGNORE
                Children [
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(TOOLTIP_ARROW_SIZE),
                            height: px(TOOLTIP_ARROW_SIZE),
                            border: px(1),
                            border_radius: px(1),
                        }
                        FeathersTooltipArrow
                        Pickable::IGNORE
                        UiTransform::from_rotation(Rot2::FRAC_PI_4)
                        ThemeBackgroundColor(tokens::TOOLTIP_BG)
                        ThemeBorderColor(tokens::TOOLTIP_BORDER)
                    )
                ]
            )
        ]
    }
}

fn arrow_cross_axis_center(root_extent: f32) -> f32 {
    let minimum = TOOLTIP_ARROW_SIZE * 0.5 + TOOLTIP_ARROW_EDGE_MARGIN;
    let maximum = (root_extent - minimum).max(minimum);
    (root_extent * 0.5).clamp(minimum, maximum)
}

fn position_tooltip_arrow(
    roots: Query<(
        &ComputedNode,
        &ResolvedPopoverPlacement,
        &FeathersTooltipRoot,
    )>,
    mut clips: Query<
        (&ChildOf, &Children, &mut Node),
        (
            With<FeathersTooltipArrowClip>,
            Without<FeathersTooltipArrow>,
        ),
    >,
    mut arrows: Query<
        &mut Node,
        (
            With<FeathersTooltipArrow>,
            Without<FeathersTooltipArrowClip>,
        ),
    >,
) {
    for (parent, children, mut clip_node) in &mut clips {
        let Ok((root_node, resolved, root)) = roots.get(parent.parent()) else {
            continue;
        };
        let Some(placement) = resolved.0.filter(|_| root.arrow) else {
            if clip_node.width != px(0) || clip_node.height != px(0) {
                clip_node.width = px(0);
                clip_node.height = px(0);
            }
            continue;
        };
        let Some(arrow_entity) = children.iter().find(|entity| arrows.contains(*entity)) else {
            continue;
        };

        let scale = root_node.inverse_scale_factor;
        let root_size = root_node.size() * scale;
        let half = TOOLTIP_ARROW_SIZE * 0.5;
        let center_x = arrow_cross_axis_center(root_size.x);
        let center_y = arrow_cross_axis_center(root_size.y);
        let border = root_node.border();
        let border_min = border.min_inset * scale;
        let border_max = border.max_inset * scale;

        let (clip_left, clip_top, clip_width, clip_height, arrow_left, arrow_top) =
            match placement.side {
                PopoverSide::Top => (
                    center_x - half,
                    root_size.y - border_max.y,
                    TOOLTIP_ARROW_SIZE,
                    TOOLTIP_ARROW_SIZE + border_max.y,
                    0.0,
                    -half + border_max.y,
                ),
                PopoverSide::Bottom => (
                    center_x - half,
                    -TOOLTIP_ARROW_SIZE,
                    TOOLTIP_ARROW_SIZE,
                    TOOLTIP_ARROW_SIZE + border_min.y,
                    0.0,
                    half,
                ),
                PopoverSide::Left => (
                    root_size.x - border_max.x,
                    center_y - half,
                    TOOLTIP_ARROW_SIZE + border_max.x,
                    TOOLTIP_ARROW_SIZE,
                    -half + border_max.x,
                    0.0,
                ),
                PopoverSide::Right => (
                    -TOOLTIP_ARROW_SIZE,
                    center_y - half,
                    TOOLTIP_ARROW_SIZE + border_min.x,
                    TOOLTIP_ARROW_SIZE,
                    half,
                    0.0,
                ),
            };

        if clip_node.left != px(clip_left)
            || clip_node.top != px(clip_top)
            || clip_node.width != px(clip_width)
            || clip_node.height != px(clip_height)
        {
            clip_node.left = px(clip_left);
            clip_node.top = px(clip_top);
            clip_node.width = px(clip_width);
            clip_node.height = px(clip_height);
        }

        if let Ok(mut arrow_node) = arrows.get_mut(arrow_entity)
            && (arrow_node.left != px(arrow_left) || arrow_node.top != px(arrow_top))
        {
            arrow_node.left = px(arrow_left);
            arrow_node.top = px(arrow_top);
        }
    }
}

fn tooltip_text(
    target: Entity,
    tooltip: &Tooltip,
    detail: TooltipDetail,
    accessibility: Option<&AccessibilityNode>,
    name: Option<&Name>,
) -> String {
    match &tooltip.content {
        TooltipContent::Text(text) => text.clone(),
        TooltipContent::Detailed { summary, details } => match detail {
            TooltipDetail::Summary => summary.clone(),
            TooltipDetail::Details => format!("{summary}\n{details}"),
        },
        TooltipContent::Auto => accessibility
            .and_then(|node| node.label())
            .map(str::to_owned)
            .or_else(|| name.map(|name| name.as_str().to_owned()))
            .unwrap_or_else(|| {
                warn!(
                    ?target,
                    "TooltipContent::Auto requires an accessibility label or Name"
                );
                format!("Tooltip missing for {target}")
            }),
    }
}

fn on_show_tooltip(
    trigger: On<ShowTooltip>,
    targets: Query<(&Tooltip, Option<&AccessibilityNode>, Option<&Name>)>,
    mut roots: ResMut<TooltipRoots>,
    mut commands: Commands,
) {
    let target = trigger.target;
    let Ok((tooltip, accessibility, name)) = targets.get(target) else {
        return;
    };
    let text = tooltip_text(target, tooltip, trigger.detail, accessibility, name);

    if let Some(entity) = roots.0.remove(&target)
        && let Ok(mut entity) = commands.get_entity(entity)
    {
        entity.despawn();
    }

    let entity = commands
        .spawn_scene(tooltip_scene(text, tooltip.placement, tooltip.arrow))
        .id();
    commands
        .entity(entity)
        .insert((ChildOf(target), TooltipPopup { target }));
    roots.0.insert(target, entity);
}

fn on_hide_tooltip(
    trigger: On<HideTooltip>,
    mut roots: ResMut<TooltipRoots>,
    mut commands: Commands,
) {
    if let Some(entity) = roots.0.remove(&trigger.target)
        && let Ok(mut entity) = commands.get_entity(entity)
    {
        entity.despawn();
    }
}

/// Feathers styling and rendering for [`Tooltip`].
pub struct FeathersTooltipPlugin;

impl Plugin for FeathersTooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        if !app.is_plugin_added::<TooltipPlugin>() {
            app.add_plugins(TooltipPlugin);
        }
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }
        app.init_resource::<TooltipRoots>()
            .add_observer(on_show_tooltip)
            .add_observer(on_hide_tooltip)
            .add_systems(PostUpdate, position_tooltip_arrow.after(UiSystems::Layout));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_scene::ScenePlugin;
    use bevy_text::Font;

    #[test]
    fn simple_tooltip_uses_static_text() {
        let target = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            tooltip_text(
                target,
                &Tooltip {
                    content: TooltipContent::Text("Save".into()),
                    ..Tooltip::default()
                },
                TooltipDetail::Summary,
                None,
                None,
            ),
            "Save"
        );
    }

    #[test]
    fn auto_prefers_accessibility_label() {
        let target = Entity::from_raw_u32(1).unwrap();
        let mut node = accesskit::Node::new(accesskit::Role::Button);
        node.set_label("Accessible");
        let accessibility = AccessibilityNode(node);
        let name = Name::new("Fallback");

        assert_eq!(
            tooltip_text(
                target,
                &Tooltip::default(),
                TooltipDetail::Summary,
                Some(&accessibility),
                Some(&name),
            ),
            "Accessible"
        );
    }

    #[test]
    fn auto_falls_back_to_name() {
        let target = Entity::from_raw_u32(1).unwrap();
        let name = Name::new("Save changes");
        assert_eq!(
            tooltip_text(
                target,
                &Tooltip::default(),
                TooltipDetail::Summary,
                None,
                Some(&name),
            ),
            "Save changes"
        );
    }

    #[test]
    fn detailed_tooltip_extends_the_summary() {
        let target = Entity::from_raw_u32(1).unwrap();
        let tooltip = Tooltip {
            content: TooltipContent::Detailed {
                summary: "Summary".into(),
                details: "Details".into(),
            },
            ..Tooltip::default()
        };

        assert_eq!(
            tooltip_text(target, &tooltip, TooltipDetail::Summary, None, None),
            "Summary"
        );
        assert_eq!(
            tooltip_text(target, &tooltip, TooltipDetail::Details, None, None),
            "Summary\nDetails"
        );
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
        ));
        app.init_asset::<Font>();
        app.init_resource::<TooltipRoots>();
        app.add_observer(on_show_tooltip);
        app.add_observer(on_hide_tooltip);
        app
    }

    #[test]
    fn preferred_placement_is_first_candidate() {
        let preferred = PopoverPlacement {
            side: PopoverSide::Right,
            align: bevy_ui_widgets::popover::PopoverAlign::End,
            gap: 12.0,
        };

        let candidates = placement_candidates(preferred);

        assert_eq!(candidates[0], preferred);
        assert_eq!(candidates[1].side, PopoverSide::Left);
        assert_eq!(candidates[2].side, PopoverSide::Top);
        assert_eq!(candidates[3].side, PopoverSide::Bottom);
    }

    #[test]
    fn multiple_targets_keep_independent_roots() {
        let mut app = test_app();

        let first = app
            .world_mut()
            .spawn(Tooltip {
                content: TooltipContent::Text("First".into()),
                ..Tooltip::default()
            })
            .id();
        let second = app
            .world_mut()
            .spawn(Tooltip {
                content: TooltipContent::Text("Second".into()),
                placement: PopoverPlacement {
                    side: PopoverSide::Right,
                    align: bevy_ui_widgets::popover::PopoverAlign::End,
                    gap: 12.0,
                },
                arrow: false,
                ..Tooltip::default()
            })
            .id();

        app.world_mut().trigger(ShowTooltip::summary(first));
        app.world_mut().trigger(ShowTooltip::summary(second));
        app.world_mut().flush();

        let roots = app.world().resource::<TooltipRoots>();
        let first_root = roots.0[&first];
        let second_root = roots.0[&second];
        assert_ne!(first_root, second_root);
        assert!(app.world().get_entity(first_root).is_ok());
        assert!(app.world().get_entity(second_root).is_ok());
        assert_eq!(
            app.world().get::<TooltipPopup>(second_root),
            Some(&TooltipPopup { target: second })
        );
        assert!(app
            .world()
            .get::<bevy_ui::RelativeCursorPosition>(second_root)
            .is_some());
        assert_eq!(
            app.world().get::<Pickable>(second_root),
            Some(&Pickable::IGNORE)
        );
        assert_eq!(
            app.world().get::<ChildOf>(second_root).map(ChildOf::parent),
            Some(second)
        );
        assert!(
            !app.world()
                .get::<FeathersTooltipRoot>(second_root)
                .unwrap()
                .arrow
        );
        assert_eq!(
            app.world().get::<Popover>(second_root).unwrap().positions,
            placement_candidates(app.world().get::<Tooltip>(second).unwrap().placement)
        );

        let arrow_clips = app
            .world()
            .iter_entities()
            .filter(EntityRef::contains::<FeathersTooltipArrowClip>)
            .map(|entity| entity.id())
            .collect::<Vec<_>>();
        let arrows = app
            .world()
            .iter_entities()
            .filter(EntityRef::contains::<FeathersTooltipArrow>)
            .map(|entity| entity.id())
            .collect::<Vec<_>>();

        assert_eq!(arrow_clips.len(), 2);
        assert_eq!(arrows.len(), 2);
        assert_eq!(
            app.world().get::<ResolvedPopoverPlacement>(second_root),
            Some(&ResolvedPopoverPlacement(None))
        );

        app.world_mut().trigger(HideTooltip { target: first });
        app.world_mut().flush();

        assert!(app.world().get_entity(first_root).is_err());
        assert!(app.world().get_entity(second_root).is_ok());
        assert!(!app
            .world()
            .resource::<TooltipRoots>()
            .0
            .contains_key(&first));
        assert_eq!(
            app.world().resource::<TooltipRoots>().0.get(&second),
            Some(&second_root)
        );
    }

    #[test]
    fn showing_the_same_target_replaces_only_its_root() {
        let mut app = test_app();
        let target = app.world_mut().spawn(Tooltip::default()).id();

        app.world_mut().trigger(ShowTooltip::summary(target));
        let first_root = app.world().resource::<TooltipRoots>().0[&target];
        app.world_mut().trigger(ShowTooltip::summary(target));
        let second_root = app.world().resource::<TooltipRoots>().0[&target];
        app.world_mut().flush();

        assert_ne!(first_root, second_root);
        assert!(app.world().get_entity(first_root).is_err());
        assert!(app.world().get_entity(second_root).is_ok());
    }
}
