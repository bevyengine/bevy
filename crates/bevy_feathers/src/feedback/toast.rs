use std::time::Duration;

use bevy_app::{Plugin, PreUpdate};
use bevy_asset::AssetServer;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::HookContext,
    observer::On,
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Commands, Query, Res},
    template::{FromTemplate, ScopedEntityIndex, template},
    world::DeferredWorld,
};
use bevy_picking::Pickable;
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{prelude::*, template_value};
use bevy_text::{FontWeight, LineBreak, TextColor, TextLayout};
use bevy_time::{Fixed, Time, Timer, TimerMode};
use bevy_ui::{
    percent, px,
    widget::{ImageNode, Text},
    AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node, Overflow, PositionType,
    UiRect,
};
use bevy_ui_widgets::{Activate, Button};

use crate::{
    constants::{fonts, icons, size},
    font_styles::InheritableFont,
    palette,
    rounded_corners::RoundedCorners,
    theme::ThemedText,
};

const TOAST_HEIGHT: f32 = 60.0;
const TOAST_MARGIN: f32 = 10.0;

/// Keeps track of currently spawned toasts in their respective positions.
///
/// This is used to determine
/// - initial positioning for each toast relative to other toasts in the same position and
/// - which toasts need to be repositioned when a toast is despawned
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct ToastPositions(pub HashMap<ToastPosition, Vec<Entity>>);

/// Severity variants for toasts. This determines the background and text color of the toast.
#[derive(Component, Default, Clone, Copy, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component, Clone, Default)]
pub enum ToastVariant {
    /// Uses [palette::INFO] for background and [palette::WHITE] for text color.
    #[default]
    Info,
    /// Uses [palette::SUCCESS] for background and [palette::WHITE] for text color.
    Success,
    /// Uses [palette::WARNING] for background and [palette::WHITE] for text color.
    Warning,
    /// Uses [palette::ERROR] for background and [palette::WHITE] for text color.
    Error,
}

/// Available positions for toasts.
#[derive(Component, Clone, Default, Reflect, Debug, PartialEq, Eq, Hash)]
#[reflect(Component, Clone, Default)]
#[component(on_add, on_despawn)]
pub enum ToastPosition {
    /// Bottom right corner of the screen. Siblings stack upwards.
    #[default]
    BottomRight,
    /// Bottom left corner of the screen. Siblings stack upwards.
    BottomLeft,
    /// Top left corner of the screen. Siblings stack downwards.
    TopLeft,
    /// Top right corner of the screen. Siblings stack downwards.
    TopRight,
}

impl ToastPosition {
    fn on_add(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        let position = world
            .entity(entity)
            .get::<ToastPosition>()
            .expect("ToastPosition should be present in ToastPosition on_add")
            .clone();
        let mut toast_positions = world.resource_mut::<ToastPositions>();
        let idx = toast_positions.0.get(&position).unwrap_or(&vec![]).len();
        toast_positions
            .0
            .entry(position.clone())
            .or_default()
            .push(entity);
        let mut entity_mut = world.entity_mut(entity);
        let mut node = entity_mut
            .get_mut::<Node>()
            .expect("Node should be present in ToastPosition on_add");
        let offset = (TOAST_HEIGHT + TOAST_MARGIN) * idx as f32;
        match position {
            ToastPosition::BottomRight => {
                node.bottom = px(offset);
                node.right = px(10.0);
            }
            ToastPosition::BottomLeft => {
                node.bottom = px(offset);
                node.left = px(10.0);
            }
            ToastPosition::TopLeft => {
                node.top = px(offset);
                node.left = px(10.0);
            }
            ToastPosition::TopRight => {
                node.top = px(offset);
                node.right = px(10.0);
            }
        }
    }

    fn on_despawn(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        let position = world
            .entity(entity)
            .get::<ToastPosition>()
            .expect("ToastPosition component should be present on entity when on_despawn is called")
            .clone();
        let mut toast_positions = world.resource_mut::<ToastPositions>();
        let removed_idx = toast_positions
            .0
            .get(&position)
            .unwrap()
            .iter()
            .position(|e| *e == entity)
            .unwrap();
        if let Some(entities) = toast_positions.0.get_mut(&position) {
            entities.retain(|&e| e != entity);
        }
        let dirty_nodes = toast_positions
            .0
            .get(&position)
            .unwrap()
            .iter()
            .skip(removed_idx)
            .cloned()
            .collect::<Vec<_>>();
        for (idx, entity) in dirty_nodes.iter().enumerate() {
            let mut entity_mut = world.entity_mut(*entity);
            let mut node = entity_mut
                .get_mut::<Node>()
                .expect("Node should be present in ToastPosition on_despawn");
            let offset = (TOAST_HEIGHT + TOAST_MARGIN) * (removed_idx + idx) as f32;
            match position {
                ToastPosition::BottomRight => {
                    node.bottom = px(-offset);
                }
                ToastPosition::BottomLeft => {
                    node.bottom = px(-offset);
                }
                ToastPosition::TopLeft => {
                    node.top = px(offset);
                }
                ToastPosition::TopRight => {
                    node.top = px(offset);
                }
            }
        }
    }
}

/// Component for keeping track of the progress bar of a toast with a duration.
/// This is used to update the width of the progress bar and despawn the toast when the timer finishes.
#[derive(Component, FromTemplate, Clone, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component, Clone)]
pub struct ToastProgressBar {
    /// [Timer] for the toast duration. The progress bar width is updated based on the remaining time of this timer, and the toast is despawned when this timer finishes.
    pub timer: Timer,
    /// The root entity of the toast. This is used to despawn the toast when the timer finishes.
    pub root_entity: Entity
}

/// A toast widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`FeathersToastProps`].`]
#[derive(SceneComponent, Default, Clone)]
#[scene(FeathersToastProps)]
pub struct FeathersToast;

/// Props used for construct a [`FeathersToast`] scene.
pub struct FeathersToastProps {
    /// The message to display in the toast.
    pub message: String,
    /// The severity variant of the toast, which determines the background and text color.
    pub variant: ToastVariant,
    /// Optional duration for the toast. If `Some`, a progress bar is shown and the toast is automatically despawned after the duration. If `None`, the toast will stay until manually despawned.
    pub duration: Option<Duration>,
    /// The position of the toast on the screen, which determines where the toast is spawned and how it is stacked with other toasts.
    pub position: ToastPosition,
}

impl Default for FeathersToastProps {
    fn default() -> Self {
        Self {
            message: "".to_string(), // TODO: Could multiline messages be supported by passing a [`SceneList`]?
            variant: ToastVariant::default(),
            duration: Some(Duration::from_secs(3)),
            position: ToastPosition::default(),
        }
    }
}

impl FeathersToast {
    fn scene(props: FeathersToastProps) -> impl Scene {
    bsn! {
        #Toast
        Node {
            width: px(300),
            height: px(60),
            margin: UiRect::all(px(5)),
            padding: UiRect::all(px(10)),
            border_radius: {RoundedCorners::All.to_border_radius(4.0)},
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute
        }
        Pickable::IGNORE
        template_value(props.variant)
        template_value(props.position)
        template(move |_| {
            let background_color = match props.variant {
                ToastVariant::Info => palette::INFO,
                ToastVariant::Success => palette::SUCCESS,
                ToastVariant::Warning => palette::WARNING,
                ToastVariant::Error => palette::ERROR,
            };
            Ok(BackgroundColor(background_color))
        })
        Children[(
            Node {
                width: percent(90),
                overflow: Overflow::clip_x()
            }
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::COMPACT_FONT,
                weight: FontWeight::NORMAL,
            }
            Children[(
                Text({props.message})
                ThemedText
                TextLayout {linebreak: LineBreak::NoWrap}
                template(move |_| {
                    let text_color = match props.variant {
                        ToastVariant::Info => palette::WHITE,
                        ToastVariant::Success => palette::WHITE,
                        ToastVariant::Warning => palette::WHITE,
                        ToastVariant::Error => palette::WHITE,
                    };
                    Ok(TextColor(text_color))
                })
            )]
        ), ({ if props.duration.is_some() {
                Box::new(bsn! {
                    Node {
                        width: px(30),
                        height: px(30),
                    }
                    Button
                    template(|ctx| {
                        let handle = ctx.resource::<AssetServer>().load(icons::X);
                        Ok(ImageNode::new(handle))
                    })
                    on(|trigger: On<Activate>, mut commands: Commands, child_of: Query<&ChildOf>| {
                        if let Ok(parent) = child_of.get(trigger.entity) {
                            commands.entity(parent.0).despawn();
                        }
                    })
                }) as Box<dyn Scene>
            } else {
                Box::new(bsn!()) as Box<dyn Scene>
            }
        }), ({ if props.duration.is_some() {
                Box::new(bsn! {
                    Node {
                        width: percent(100),
                        height: px(10),
                        position_type: PositionType::Absolute,
                        bottom: px(0),
                        left: px(0),
                    }
                    BackgroundColor(palette::WHITE)
                    template(move |ctx| {
                        let root_entity = ctx.get_scoped_entity(ScopedEntityIndex { scope: 1, index: 0}); // TODO: Why is the scope 1 here? Before #24008 this was in 0.
                        Ok(ToastProgressBar { timer: Timer::new(props.duration.unwrap(), TimerMode::Once), root_entity })
                    })
                    // ToastProgressBar { timer: Timer::new(props.duration.unwrap(), TimerMode::Once), root_entity: #Toast } // TODO: This panics if the EntityReference is there
                    }) as Box<dyn Scene>
            } else {
                Box::new(bsn!()) as Box<dyn Scene>
            }
        })]
    }
    }
}

fn tick_toasts_progress_bars(
    mut commands: Commands,
    mut toast_progress_bars: Query<(&mut Node, &mut ToastProgressBar)>,
    time: Res<Time<Fixed>>,
) {
    for (mut node, mut toast_progress_bar) in &mut toast_progress_bars {
        let timer = &mut toast_progress_bar.timer;
        timer.tick(time.delta());
        let remaining_secs = timer.remaining_secs();
        let duration_secs = timer.duration().as_secs() as f32;
        let remaining = remaining_secs / duration_secs;
        node.width = percent(remaining * 100.);
        if timer.is_finished() {
            commands.entity(toast_progress_bar.root_entity).despawn();
        }
    }
}

/// Plugin which registers the systems for managing toasts.
pub struct ToastsPlugin;

impl Plugin for ToastsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ToastPositions>();
        app.add_systems(PreUpdate, tick_toasts_progress_bars);
    }
}
