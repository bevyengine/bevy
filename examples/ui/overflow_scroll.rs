//! Shows the distinction between [`Overflow::visible`], [`Overflow::scroll`], and [`Overflow::scroll_no_clip`].
//!
//! Use [`KeyCode::ArrowLeft`] or [`KeyCode::ArrowRight`] to scroll the items

use bevy::{
    app::{App, Startup, Update},
    asset::{AssetServer, Handle},
    core_pipeline::core_2d::Camera2d,
    ecs::{
        bundle::Bundle,
        children,
        component::Component,
        hierarchy::{ChildOf, Children},
        query::With,
        spawn::{SpawnIter, SpawnRelated, SpawnableList},
        system::{Commands, Query, Res},
    },
    image::{Image, TextureAtlas, TextureAtlasLayout},
    input::{keyboard::KeyCode, ButtonInput},
    math::UVec2,
    time::Time,
    ui::{
        widget::{ImageNode, Text},
        AlignItems, AlignSelf, FlexDirection, JustifySelf, Node, Overflow, ScrollPosition, Val,
    },
    DefaultPlugins,
};

/// Length of the sides of the texture
const TEXTURE_SIDES: u32 = 64;
/// Length of the sides of the image nodes
const NODE_SIDES: f32 = 64. * 2.;
/// Speed of scrolling in pixels per second
const SCROLL_SPEED: f32 = 320.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, scroll)
        .run();
}

#[derive(Component)]
/// Marks UI [`Node`] that can be scrolled
struct Scrollable;

/// Spawns a scene with 3 UI nodes
///
/// [`Overflow::visible`] allows items to be visible, even those out of the bounds of the node  
/// [`Overflow::scroll`] clips items out of the bounds but allows scrolling using [`ScrollPosition`]
/// [`Overflow::scroll`] allows scrolling using [`ScrollPosition`] and keeps items out of bounds visible  
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load("textures/food_kenney.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(TEXTURE_SIDES), 7, 6, None, None);
    let layout_handle = asset_server.add(layout);

    commands.spawn((
        Node {
            width: Val::Vw(80.),
            flex_direction: FlexDirection::Column,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..Default::default()
        },
        Children::spawn(ExampleNodes(image, layout_handle)),
    ));
}

/// Scrolls the UI nodes on [`KeyCode::ArrowRight`] or [`KeyCode::ArrowLeft`] inputs
fn scroll(
    mut scrollables: Query<&mut ScrollPosition, With<Scrollable>>,
    key_codes: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let to_scroll: f32 = [
        Some(SCROLL_SPEED).filter(|_| key_codes.pressed(KeyCode::ArrowRight)),
        Some(-SCROLL_SPEED).filter(|_| key_codes.pressed(KeyCode::ArrowLeft)),
    ]
    .into_iter()
    .flatten()
    .sum();
    let scaled_scroll = to_scroll * time.delta_secs();

    for mut scrollable in scrollables.iter_mut() {
        // No need to deal with going beyond the end of the scroll as it is clamped by Bevy
        scrollable.offset_x += scaled_scroll;
    }
}

struct ExampleNodes(Handle<Image>, Handle<TextureAtlasLayout>);

impl SpawnableList<ChildOf> for ExampleNodes {
    fn spawn(self, world: &mut bevy_ecs::world::World, entity: bevy_ecs::entity::Entity) {
        for (header, overflow) in [
            ("Overflow::visible", Overflow::visible()),
            ("Overflow::scroll", Overflow::scroll()),
            ("Overflow::scroll_no_clip", Overflow::scroll_no_clip()),
        ] {
            world.spawn((
                Node {
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                children![
                    Text::new(header),
                    (
                        Scrollable,
                        Node {
                            width: Val::Px(NODE_SIDES),
                            height: Val::Px(NODE_SIDES),
                            flex_direction: FlexDirection::Row,
                            overflow,
                            ..Default::default()
                        },
                        spawn_list(&self.0, &self.1)
                    )
                ],
                ChildOf(entity),
            ));
        }
    }

    fn size_hint(&self) -> usize {
        3
    }
}

/// Spawn a list of node, each with a distinct index into a texture atlas
fn spawn_list(image: &Handle<Image>, layout: &Handle<TextureAtlasLayout>) -> impl Bundle {
    Children::spawn(SpawnIter(
        (0..(6 * 7))
            .map(|id| ImageNode {
                image: image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: layout.clone(),
                    index: id,
                }),
                ..Default::default()
            })
            .collect::<Vec<_>>()
            .into_iter(),
    ))
}
