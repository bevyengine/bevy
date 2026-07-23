//! Demonstrates how Display and Visibility work in the UI.

use bevy::{
    color::palettes::css::{DARK_CYAN, DARK_GRAY, YELLOW},
    ecs::{component::Mutable, template::EntityTemplate},
    prelude::*,
    text::FontSourceTemplate,
};

const PALETTE: [&str; 4] = ["27496D", "466B7A", "669DB3", "ADCBE3"];
const HIDDEN_COLOR: Color = Color::srgb(1.0, 0.7, 0.7);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                buttons_handler::<Display>,
                buttons_handler::<Visibility>,
                text_hover,
            ),
        )
        .run();
}

/// A component attached to a button that implements `TargetUpdate`.
/// Activating the button will alter the target entity's
/// `T` component.
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

trait TargetUpdate {
    type TargetComponent: Component<Mutability = Mutable>;
    const NAME: &'static str;
    fn update_target(&self, target: &mut Self::TargetComponent) -> String;
}

impl TargetUpdate for Target<Display> {
    type TargetComponent = Node;
    const NAME: &'static str = "Display";
    fn update_target(&self, node: &mut Self::TargetComponent) -> String {
        node.display = match node.display {
            Display::Flex => Display::None,
            Display::None => Display::Flex,
            Display::Block | Display::Grid => unreachable!(),
        };
        format!("{}::{:?} ", Self::NAME, node.display)
    }
}

impl TargetUpdate for Target<Visibility> {
    type TargetComponent = Visibility;
    const NAME: &'static str = "Visibility";
    fn update_target(&self, visibility: &mut Self::TargetComponent) -> String {
        *visibility = match *visibility {
            Visibility::Inherited => Visibility::Visible,
            Visibility::Visible => Visibility::Hidden,
            Visibility::Hidden => Visibility::Inherited,
        };
        format!("{}::{visibility:?}", Self::NAME)
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let palette: [Color; 4] = PALETTE.map(|hex| Srgba::hex(hex).unwrap().into());

    let text_font = TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
        ..default()
    };

    commands.spawn(Camera2d);

    let panels_child = commands
        .spawn_scene(panels("fonts/FiraSans-Bold.ttf", &palette))
        .id();

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
fn panels(text_font: &'static str, palette: &[Color; 4]) -> impl Scene {
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
                        button_scene::<Display>(text_font, #LeftGrandParent),
                        button_scene::<Visibility>(text_font, #LeftGrandParent),

                        #RightParent
                        right_panel_node_base(400, 1)
                        Children [
                            button_scene::<Display>(text_font, #LeftParent),
                            button_scene::<Visibility>(text_font, #LeftParent),

                            #RightChild
                            right_panel_node_base(300, 2)
                            Children [
                                button_scene::<Display>(text_font, #LeftChild),
                                button_scene::<Visibility>(text_font, #LeftChild),

                                #RightGrandChild
                                right_panel_node_base(200, 3)
                                Children [
                                    button_scene::<Display>(text_font, #LeftGrandChild),
                                    button_scene::<Visibility>(text_font, #LeftGrandChild),

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

/// A button that, when pressed, will execute up a target action on the provided `target`.
/// The target action is applied to the `T` component on the `target.`
fn button_scene<T>(text_font: &'static str, target: EntityTemplate) -> impl Scene
where
    T: Default + Clone + Unpin + std::fmt::Debug + Send + Sync + 'static,
    Target<T>: TargetUpdate,
{
    bsn! {
        Button
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
        Children [
            Text(format!("{}::{:?}", Target::<T>::NAME, T::default()))
            TextFont {
                font: FontSourceTemplate::Handle(text_font)
            }
            TextLayout::justify(Justify::Center)
        ]
    }
}

fn buttons_handler<T>(
    mut left_panel_query: Query<&mut <Target<T> as TargetUpdate>::TargetComponent>,
    mut visibility_button_query: Query<(&Target<T>, &Interaction, &Children), Changed<Interaction>>,
    mut text_query: Query<(&mut Text, &mut TextColor)>,
) where
    T: Send + Sync,
    Target<T>: TargetUpdate + Component,
{
    for (target, interaction, children) in visibility_button_query.iter_mut() {
        if matches!(interaction, Interaction::Pressed) {
            let mut target_value = left_panel_query.get_mut(target.id.unwrap()).unwrap();
            for &child in children {
                if let Ok((mut text, mut text_color)) = text_query.get_mut(child) {
                    **text = target.update_target(target_value.as_mut());
                    text_color.0 = if text.contains("None") || text.contains("Hidden") {
                        Color::srgb(1.0, 0.7, 0.7)
                    } else {
                        Color::WHITE
                    };
                }
            }
        }
    }
}

fn text_hover(
    mut button_query: Query<(&Interaction, &mut BackgroundColor, &Children), Changed<Interaction>>,
    mut text_query: Query<(&Text, &mut TextColor)>,
) {
    for (interaction, mut color, children) in button_query.iter_mut() {
        match interaction {
            Interaction::Hovered => {
                *color = Color::BLACK.with_alpha(0.6).into();
                for &child in children {
                    if let Ok((_, mut text_color)) = text_query.get_mut(child) {
                        // Bypass change detection to avoid recomputation of the text when only changing the color
                        text_color.bypass_change_detection().0 = YELLOW.into();
                    }
                }
            }
            _ => {
                *color = Color::BLACK.with_alpha(0.5).into();
                for &child in children {
                    if let Ok((text, mut text_color)) = text_query.get_mut(child) {
                        text_color.bypass_change_detection().0 =
                            if text.contains("None") || text.contains("Hidden") {
                                HIDDEN_COLOR
                            } else {
                                Color::WHITE
                            };
                    }
                }
            }
        }
    }
}
