//! Demonstrates how Display and Visibility work in the UI.

use bevy::{
    color::palettes::css::{DARK_CYAN, DARK_GRAY},
    ecs::{component::Mutable, template::EntityTemplate},
    feathers::{
        controls::{FeathersListRow, FeathersSelect, OptionIndex},
        display::caption,
        theme::UiTheme,
        FeathersPlugins,
    },
    prelude::*,
    ui::Selected,
    ui_widgets::ValueChange,
};

#[path = "../../helpers/theme.rs"]
mod theme;

const PALETTE: [&str; 4] = ["27496D", "466B7A", "669DB3", "ADCBE3"];
const HIDDEN_COLOR: Color = Color::srgb(1.0, 0.7, 0.7);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(theme::basic_example_theme(Color::WHITE)))
        .add_systems(Startup, setup)
        .add_observer(on_value_change::<NodeDisplaySetting>)
        .add_observer(on_value_change::<NodeVisibilitySetting>)
        .run();
}

/// A component attached to an option select that will change the target entity
/// with a given setting `T`.
/// The way the change is implemented is via the `TargetUpdate<T>` trait. Changing the
/// value of the option select will alter the target entity.
#[derive(Component, Clone, Default)]
struct Target<T> {
    id: Option<Entity>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> Target<T> {
    fn new(id: Entity) -> Self {
        Self {
            id: Some(id),
            phantom: std::marker::PhantomData,
        }
    }
}

/// The trait to be used in conjunction with the `Target` struct.
/// This trait specifies the `TargetComponent` which will be modified.
/// `T` is a setting option type that will influence how the `TargetComponent` is updated.
trait TargetUpdate<T: Component + Clone + Default + PartialEq> {
    type TargetComponent: Component<Mutability = Mutable>;
    fn update_target(&self, target: &mut Self::TargetComponent, value: &T);
}

#[derive(Component, Clone, Debug, Default, PartialEq)]
enum NodeDisplaySetting {
    #[default]
    Flex,
    None,
}

#[derive(Component, Clone, Debug, Default, PartialEq)]
enum NodeVisibilitySetting {
    #[default]
    Inherited,
    Visible,
    Hidden,
}

/// For `Display`, this impl of `TargetUpdate` will affect this entity's `Node` component's display property.
impl TargetUpdate<NodeDisplaySetting> for Target<NodeDisplaySetting> {
    type TargetComponent = Node;
    fn update_target(&self, node: &mut Self::TargetComponent, value: &NodeDisplaySetting) {
        node.display = match value {
            NodeDisplaySetting::Flex => Display::Flex,
            NodeDisplaySetting::None => Display::None,
        };
    }
}

/// For `Visibility`, this impl of `TargetUpdate` will affect this entity's `Visibility` component.`
impl TargetUpdate<NodeVisibilitySetting> for Target<NodeVisibilitySetting> {
    type TargetComponent = Visibility;
    fn update_target(&self, visibility: &mut Self::TargetComponent, value: &NodeVisibilitySetting) {
        *visibility = match value {
            NodeVisibilitySetting::Inherited => Visibility::Inherited,
            NodeVisibilitySetting::Visible => Visibility::Visible,
            NodeVisibilitySetting::Hidden => Visibility::Hidden,
        };
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let palette: [Color; 4] = PALETTE.map(|hex| Srgba::hex(hex).unwrap().into());

    let text_font = TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
        ..default()
    };

    commands.spawn(Camera2d);

    let panels_child = commands.spawn_scene(panels(&palette)).id();

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .add_child(panels_child)
        .with_children(|parent| {
            parent.spawn((
                Text::new("Use the panel on the right to change the Display and Visibility properties for the respective nodes of the panel on the left"),
                text_font.clone(),
                TextLayout::justify(Justify::Center),
                Node {
                    margin: UiRect::bottom(px(10)),
                    ..Default::default()
                },
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Start,
                    justify_content: JustifyContent::Start,
                    column_gap: px(10),
                    ..default()
                })
                .with_children(|builder| {
                    let text_font = TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                        ..default()
                    };

                    builder.spawn((
                        Text::new("Display::None\nVisibility::Hidden\nVisibility::Inherited"),
                        text_font.clone(),
                        TextColor(HIDDEN_COLOR),
                        TextLayout::justify(Justify::Center),
                    ));
                    builder.spawn((
                        Text::new("-\n-\n-"),
                        text_font.clone(),
                        TextColor(DARK_GRAY.into()),
                        TextLayout::justify(Justify::Center),
                    ));
                    builder.spawn((Text::new("The UI Node and its descendants will not be visible and will not be allotted any space in the UI layout.\nThe UI Node will not be visible but will still occupy space in the UI layout.\nThe UI node will inherit the visibility property of its parent. If it has no parent it will be visible."), text_font));
                });
        });
}

/// Returns the main interactable of the example as a scene.
/// The left panel changes Display and Visibility based on actions taken
/// in the right panel.
fn panels(palette: &[Color; 4]) -> impl Scene {
    let left_panel_node_base = |height_px: i32, palette_index: usize| {
        bsn! {
            Node {
                height: px(height_px),
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::FlexEnd,
            }
            BackgroundColor(palette[palette_index])
        }
    };

    let right_panel_node_base = |width_height_px: i32, palette_index: usize| {
        bsn! {
            Node {
                width: px(width_height_px),
                height: px(width_height_px),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect {
                    left: px(5),
                    top: px(5),
                },
            }
            BackgroundColor(palette[palette_index])
        }
    };

    // This must be defined as a big scene because
    // the right panel has entity references to affect look of the left panel.
    // The entity references must exist in the same `bsn!` block in order to be valid.
    bsn! {
        Node {
            width: percent(100)
        }
        Children [
            #LeftPanel
            Node {
                width: percent(50),
                height: px(520),
                justify_content: JustifyContent::Center,
            }
            Children [
                Node {
                    padding: UiRect::all(px(10)),
                }
                BackgroundColor(Color::WHITE)
                Children [
                    Node
                    BackgroundColor(Color::BLACK)
                    Children [
                        #LeftGrandParent
                        left_panel_node_base(500, 0)
                        Outline {
                            width: px(4),
                            color: DARK_CYAN,
                            offset: px(10),
                        }
                        Children [
                            Node {
                                width: px(100),
                                height: px(500),
                            },

                            #LeftParent
                            left_panel_node_base(400, 1)
                            Children [
                                Node {
                                    width: px(100),
                                    height: px(400),
                                },

                                #LeftChild
                                left_panel_node_base(300, 2)
                                Children [
                                    Node {
                                        width: px(100),
                                        height: px(300),
                                    },

                                    #LeftGrandChild
                                    Node {
                                        width: px(200),
                                        height: px(200),

                                    }
                                    BackgroundColor(palette[3])
                                ]
                            ]
                        ]
                    ]
                ]
            ],

            #RightPanel
            Node {
                width: percent(50),
                justify_content: JustifyContent::Center,
            }
            Children [
                Node {
                    padding: UiRect::all(px(10)),
                }
                BackgroundColor(Color::WHITE)
                Children [
                    #RightGrandParent
                    right_panel_node_base(500, 0)
                    Outline {
                        width: px(4),
                        color: DARK_CYAN,
                        offset: px(10),
                    }
                    Children [
                        feathers_select_display(#LeftGrandParent),
                        feathers_select_visibility(#LeftGrandParent),

                        #RightParent
                        right_panel_node_base(400, 1)
                        Children [
                            feathers_select_display(#LeftParent),
                            feathers_select_visibility(#LeftParent),

                            #RightChild
                            right_panel_node_base(300, 2)
                            Children [
                                feathers_select_display(#LeftChild),
                                feathers_select_visibility(#LeftChild),

                                #RightGrandChild
                                right_panel_node_base(200, 3)
                                Children [
                                    feathers_select_display(#LeftGrandChild),
                                    feathers_select_visibility(#LeftGrandChild),

                                    Node {
                                        width: px(100),
                                        height: px(100),
                                    }
                                ]
                            ]
                        ]
                    ]
                ]
            ],
        ]
    }
}

/// A dropdown select that will execute a target action on the provided `target` via `on_value_change`.
/// This select will update the display property on the Node of the target.
fn feathers_select_display(target: EntityTemplate) -> impl Scene {
    bsn! {
        select_base::<NodeDisplaySetting>(target)
        @FeathersSelect {
            @options: {
                bsn_list! {
                    @FeathersListRow Selected OptionIndex(0) template_value(NodeDisplaySetting::Flex) Children[caption(format!("Display::{:?}", Display::Flex))],
                    @FeathersListRow OptionIndex(1) template_value(NodeDisplaySetting::None) Children[caption(format!("Display::{:?}", Display::None))],
                }
            }
        }
    }
}

/// A dropdown select that will execute a target action on the provided `target` via `on_value_change`.
/// This select will update the Visibility component directly on the target.
fn feathers_select_visibility(target: EntityTemplate) -> impl Scene {
    bsn! {
        select_base::<NodeVisibilitySetting>(target)
        @FeathersSelect {
            @options: {
                bsn_list! {
                    @FeathersListRow Selected OptionIndex(0) template_value(NodeVisibilitySetting::Inherited) Children[caption(format!("Visibility::{:?}", Visibility::Inherited))],
                    @FeathersListRow OptionIndex(1) template_value(NodeVisibilitySetting::Visible) Children[caption(format!("Visibility::{:?}", Visibility::Visible))],
                    @FeathersListRow OptionIndex(2) template_value(NodeVisibilitySetting::Hidden) Children[caption(format!("Visibility::{:?}", Visibility::Hidden))],
                }
            }
        }
    }
}

/// Observer that reacts to value changes of a FeathersSelect for the `T` setting,
/// and updates the target entity accordingly.
fn on_value_change<T: Component + Default + Clone + PartialEq + Send + Sync>(
    event: On<ValueChange<Entity>>,
    setting_value_q: Query<&T>,
    list_box_q: Query<(&Children, &Target<T>), With<FeathersSelect>>,
    mut target_component_query: Query<&mut <Target<T> as TargetUpdate<T>>::TargetComponent>,
    mut commands: Commands,
) where
    Target<T>: TargetUpdate<T>,
{
    let Ok(value) = setting_value_q.get(event.value) else {
        return;
    };
    let Ok((children, target)) = list_box_q.get(event.source) else {
        return;
    };

    let Ok(mut target_value) = target_component_query.get_mut(target.id.unwrap()) else {
        return;
    };

    target.update_target(target_value.as_mut(), value);

    // Update selected status of children
    for child in children {
        if let Ok(child_value) = setting_value_q.get(*child)
            && *child_value == *value
        {
            commands.entity(*child).insert(Selected);
        } else {
            commands.entity(*child).remove::<Selected>();
        }
    }
}

/// A scene of the Node, BackgroundColor, and `Target<T>` for the given target.
fn select_base<T: Component + Clone + Default + PartialEq>(target: EntityTemplate) -> impl Scene
where
    T: Default + Clone + Unpin + std::fmt::Debug + Send + Sync + 'static,
    Target<T>: TargetUpdate<T>,
{
    bsn! {
        Node {
            align_self: AlignSelf::FlexStart,
            padding: UiRect::axes(px(5), px(1)),
        }
        BackgroundColor({Color::BLACK.with_alpha(0.5)})
        template(move |ctx| match target {
            EntityTemplate::Entity(ent) => Ok(Target::<T>::new(ent)),
            EntityTemplate::SceneEntityReference(scene_entity_reference) => Ok(Target::<T>::new(ctx.get_entity(scene_entity_reference))),
            EntityTemplate::None => Err(BevyError::error("Did not set up example correctly!"))
        })
    }
}
