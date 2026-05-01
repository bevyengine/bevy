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
    template::{template, FromTemplate},
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

#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct ToastPositions(pub HashMap<ToastPosition, Vec<Entity>>);

#[derive(Component, Default, Clone, Copy, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component, Clone, Default)]
pub enum ToastVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Component, Clone, Default, Reflect, Debug, PartialEq, Eq, Hash)]
#[reflect(Component, Clone, Default)]
#[component(on_add, on_despawn)]
pub enum ToastPosition {
    #[default]
    BottomRight,
    BottomLeft,
    TopLeft,
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
        let offset = 70.0 * idx as f32; // Assuming each toast has a height of 60px + 10px margin
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
            let offset = 70.0 * (removed_idx + idx) as f32; // Assuming each toast has a height of 60px + 10px margin
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

#[derive(Component, FromTemplate, Clone, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component, Clone)]
pub struct ToastProgressBar {
    pub timer: Timer,
}

pub struct ToastProps {
    pub message: String,
    pub variant: ToastVariant,
    pub duration: Option<Duration>,
    pub position: ToastPosition,
}

impl Default for ToastProps {
    fn default() -> Self {
        Self {
            message: "".to_string(),
            variant: ToastVariant::default(),
            duration: Some(Duration::from_secs(3)),
            position: ToastPosition::default(),
        }
    }
}

pub fn toast(props: ToastProps) -> impl Scene {
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
                        ToastVariant::Warning => palette::BLACK,
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
                Box::new(bsn! {(
                    Node {
                        width: percent(100),
                        height: px(10),
                        position_type: PositionType::Absolute,
                        bottom: px(0),
                        left: px(0),
                    }
                    BackgroundColor(palette::WHITE)
                    ToastProgressBar { timer: Timer::new(props.duration.unwrap(), TimerMode::Once) }
                    // ToastProgressBar { timer: Timer::new(props.duration.unwrap(), TimerMode::Once), root_entity: #Toast } // This panics if the EntityReference is there
                    )}) as Box<dyn Scene>
            } else {
                Box::new(bsn!()) as Box<dyn Scene>
            }
        })]
    }
}

fn tick_toasts_progress_bars(
    mut commands: Commands,
    mut toast_progress_bars: Query<(Entity, &mut Node, &mut ToastProgressBar)>,
    child_of: Query<&ChildOf>,
    time: Res<Time<Fixed>>,
) {
    for (entity, mut node, mut toast_progress_bar) in &mut toast_progress_bars {
        let timer = &mut toast_progress_bar.timer;
        timer.tick(time.delta());
        let remaining_secs = timer.remaining_secs();
        let duration_secs = timer.duration().as_secs() as f32;
        let remaining = remaining_secs / duration_secs;
        node.width = percent(remaining * 100.);
        let mut root_entity = entity;
        // Hacky solution to get the root entity of the toast since using EntityReference in ToastProgressBar causes a panic.
        while let Ok(entity) = child_of.get(root_entity) {
            root_entity = entity.0;
        }
        if timer.is_finished() {
            commands.entity(root_entity).despawn();
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
