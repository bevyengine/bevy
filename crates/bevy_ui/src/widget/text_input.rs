#![allow(missing_docs)]

use crate::widget::on_focused_keyboard_input;
use crate::widget::NextFocus;
use crate::widget::SingleLineInputField;
use crate::widget::TextCursorBlinkTimer;
use crate::widget::TextCursorStyle;
use crate::widget::TextInputMultiClickCounter;
use crate::widget::TextInputSubmitBehaviour;
use crate::widget::TextUnderCursorColor;
use crate::widget::MULTI_CLICK_PERIOD;
use crate::ComputedNode;
use crate::ComputedNodeTarget;
use crate::ContentSize;
use crate::Measure;
use crate::MeasureArgs;
use crate::Node;
use crate::UiGlobalTransform;
use crate::UiScale;
use crate::UiSystems;
use bevy_app::Plugin;
use bevy_app::PostUpdate;
use bevy_asset::Assets;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::observer::Observer;
use bevy_ecs::observer::On;
use bevy_ecs::query::With;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_ecs::system::ResMut;
use bevy_ecs::world::DeferredWorld;
use bevy_ecs::world::Mut;
use bevy_ecs::world::Ref;
use bevy_input_focus::tab_navigation::NavAction;
use bevy_input_focus::InputFocus;
use bevy_math::IVec2;
use bevy_math::Rect;
use bevy_math::Vec2;
use bevy_picking::events::Click;
use bevy_picking::events::Drag;
use bevy_picking::events::Move;
use bevy_picking::events::Pointer;
use bevy_picking::events::Press;
use bevy_picking::pointer::PointerButton;
use bevy_text::CosmicFontSystem;
use bevy_text::Font;
use bevy_text::Justify;
use bevy_text::SpaceAdvance;
use bevy_text::TextColor;
use bevy_text::TextFont;
use bevy_text::TextInputAction;
use bevy_text::TextInputActions;
use bevy_text::TextInputAttributes;
use bevy_text::TextInputBuffer;
use bevy_text::TextInputTarget;
use bevy_text::TextInputUndoHistory;
use bevy_text::TextInputVisibleLines;
use bevy_text::TextLayoutInfo;
use bevy_text::TextPipeline;
use bevy_text::TextSelectionBlockColor;
use bevy_time::Time;
use taffy::AvailableSpace;

pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (update_line_input_attributes, measure_line)
                .in_set(UiSystems::Content)
                .before(UiSystems::Layout),
        );
    }
}

/// Main single line text input component
#[derive(Component, Debug, Default)]
#[require(
    Node,
    TextFont,
    TextColor,
    TextSelectionBlockColor,
    TextInputMultiClickCounter,
    TextInputBuffer,
    TextInputTarget,
    TextInputAttributes,
    TextInputActions,
    TextCursorStyle,
    TextLayoutInfo,
    TextCursorBlinkTimer,
    TextInputUndoHistory,
    SpaceAdvance,
    SingleLineInputField,
    TextInputVisibleLines(1.),
    TextUnderCursorColor,
    TextInputSubmitBehaviour {
        clear_on_submit: false,
        navigate_on_submit: NextFocus::Navigate(NavAction::Next),
    },
    ContentSize
)]
#[component(
    on_add = on_add_text_input_node,
    on_remove = on_remove_input_focus,
)]
pub struct TextInput {
    pub justify: Justify,
}

fn on_add_text_input_node(mut world: DeferredWorld, context: HookContext) {
    for mut observer in [
        Observer::new(on_text_input_dragged),
        Observer::new(on_text_input_pressed),
        Observer::new(on_multi_click_set_selection),
        Observer::new(on_move_clear_multi_click),
        Observer::new(on_focused_keyboard_input),
    ] {
        observer.watch_entity(context.entity);
        world.commands().spawn(observer);
    }
}

fn on_remove_input_focus(mut world: DeferredWorld, context: HookContext) {
    let mut input_focus = world.resource_mut::<InputFocus>();
    if input_focus.0 == Some(context.entity) {
        input_focus.0 = None;
    }
}

fn on_text_input_pressed(
    trigger: On<Pointer<Press>>,
    mut node_query: Query<(&ComputedNode, &UiGlobalTransform, &mut TextInputActions)>,
    mut input_focus: ResMut<InputFocus>,
    ui_scale: Res<UiScale>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut actions)) = node_query.get_mut(trigger.target()) else {
        return;
    };

    input_focus.set(trigger.target());

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position = trigger.pointer_location.position * node.inverse_scale_factor().recip()
        / ui_scale.0
        - rect.min;

    actions.queue(TextInputAction::Click(position.as_ivec2()));
}

fn on_text_input_dragged(
    trigger: On<Pointer<Drag>>,
    mut node_query: Query<(&ComputedNode, &UiGlobalTransform, &mut TextInputActions)>,
    input_focus: Res<InputFocus>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    if !input_focus
        .0
        .is_some_and(|input_focus_entity| input_focus_entity == trigger.target())
    {
        return;
    }

    let Ok((node, transform, mut actions)) = node_query.get_mut(trigger.target()) else {
        return;
    };

    let rect = Rect::from_center_size(transform.translation, node.size());

    let position =
        trigger.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

    let target = IVec2 {
        x: position.x as i32,
        y: position.y as i32,
    };

    actions.queue(TextInputAction::Drag(target));
}

fn on_multi_click_set_selection(
    click: On<Pointer<Click>>,
    time: Res<Time>,
    mut text_input_nodes: Query<(&ComputedNode, &UiGlobalTransform, &mut TextInputActions)>,
    mut multi_click_datas: Query<&mut TextInputMultiClickCounter>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Ok((node, transform, mut actions)) = text_input_nodes.get_mut(click.target()) else {
        return;
    };

    let now = time.elapsed_secs();
    if let Ok(mut multi_click_data) = multi_click_datas.get_mut(click.target()) {
        if now - multi_click_data.last_click_time
            <= MULTI_CLICK_PERIOD * multi_click_data.click_count as f32
        {
            let rect = Rect::from_center_size(transform.translation, node.size());

            let position =
                click.pointer_location.position * node.inverse_scale_factor().recip() - rect.min;

            match multi_click_data.click_count {
                1 => {
                    multi_click_data.click_count += 1;
                    multi_click_data.last_click_time = now;

                    actions.queue(TextInputAction::DoubleClick(position.as_ivec2()));
                    return;
                }
                2 => {
                    actions.queue(TextInputAction::TripleClick(position.as_ivec2()));
                    if let Ok(mut entity) = commands.get_entity(click.target()) {
                        entity.try_remove::<TextInputMultiClickCounter>();
                    }
                    return;
                }
                _ => (),
            }
        }
    }
    if let Ok(mut entity) = commands.get_entity(click.target()) {
        entity.try_insert(TextInputMultiClickCounter {
            last_click_time: now,
            click_count: 1,
        });
    }
}

fn on_move_clear_multi_click(move_event: On<Pointer<Move>>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(move_event.target()) {
        entity.try_remove::<TextInputMultiClickCounter>();
    }
}

fn measure_line(
    _fonts: Res<Assets<Font>>,
    mut query: Query<
        (
            Entity,
            Ref<ComputedNodeTarget>,
            Ref<TextFont>,
            Mut<ContentSize>,
        ),
        (With<Node>, With<TextInput>),
    >,
    mut _text_pipeline: ResMut<TextPipeline>,
    mut _font_system: ResMut<CosmicFontSystem>,
) {
    for (_entity, target, text_font, mut content_size) in query.iter_mut() {
        if target.is_changed() || text_font.is_changed() {
            let line_height = match text_font.line_height {
                bevy_text::LineHeight::Px(px) => px,
                bevy_text::LineHeight::RelativeToFont(r) => r * text_font.font_size,
            } * target.scale_factor;

            content_size.set(crate::NodeMeasure::Custom(Box::new(LineHeightMeasure {
                line_height,
            })));
        }
    }
}

struct LineHeightMeasure {
    line_height: f32,
}

impl Measure for LineHeightMeasure {
    fn measure(&mut self, measure_args: MeasureArgs, _style: &taffy::Style) -> Vec2 {
        Vec2::new(
            measure_args
                .width
                .unwrap_or_else(|| match measure_args.available_width {
                    AvailableSpace::Definite(x) => x,
                    AvailableSpace::MinContent => 0.,
                    AvailableSpace::MaxContent => 0.,
                }),
            self.line_height,
        )
    }
}

fn update_line_input_attributes(
    mut text_input_node_query: Query<(&TextFont, &TextInput, &mut TextInputAttributes)>,
) {
    for (font, line_input, mut attributes) in text_input_node_query.iter_mut() {
        attributes.set_if_neq(TextInputAttributes {
            font: font.font.clone(),
            font_size: font.font_size,
            font_smoothing: font.font_smoothing,
            justify: line_input.justify,
            line_break: bevy_text::LineBreak::NoWrap,
            line_height: font.line_height,
            max_chars: None,
        });
    }
}
