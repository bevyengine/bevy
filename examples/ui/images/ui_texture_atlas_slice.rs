//! This example illustrates how to create buttons with their texture atlases sliced
//! and kept in proportion instead of being stretched by the button dimensions

use bevy::{
    color::palettes::css::{GOLD, ORANGE},
    picking::hover::Hovered,
    platform::collections::HashSet,
    prelude::*,
    ui::{widget::NodeImageMode, Pressed},
    ui_widgets::Button,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .run();
}

/// Updates each button label whenever its `Pressed` or `Hovered` state changes.
/// The image node, defined from a spritesheet, will also update upon pressing the button.
///
/// `Hovered` always exists on the button, so `Ref::is_changed` catches hover transitions and
/// the insertion of `Pressed`. Removal of `Pressed` is not reported by change detection, so we
/// also restyle any button that just had `Pressed` removed this frame.
fn button_system(
    mut buttons: Query<
        (
            Entity,
            Option<Ref<Pressed>>,
            Ref<Hovered>,
            &mut ImageNode,
            &Children,
        ),
        With<Button>,
    >,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut text_query: Query<&mut Text>,
) {
    // Buttons that had `Pressed` removed this frame; change detection does not report removals.
    let just_unpressed: HashSet<Entity> = removed_pressed.read().collect();
    for (entity, pressed, hovered, mut image, children) in &mut buttons {
        let changed = hovered.is_changed()
            || pressed.as_ref().is_some_and(Ref::is_changed)
            || just_unpressed.contains(&entity);
        if changed {
            set_button_style(
                pressed.is_some(),
                hovered.get(),
                &mut image,
                children,
                &mut text_query,
            );
        }
    }
}

/// Shared styling logic: pressed takes precedence over hover.
fn set_button_style(
    pressed: bool,
    hovered: bool,
    image: &mut ImageNode,
    children: &Children,
    text_query: &mut Query<&mut Text>,
) {
    let mut text = text_query.get_mut(children[0]).unwrap();
    match (pressed, hovered) {
        (true, _) => {
            **text = "Press".to_string();
            if let Some(atlas) = &mut image.texture_atlas {
                atlas.index = (atlas.index + 1) % 30;
            }
            image.color = GOLD.into();
        }
        (false, true) => {
            **text = "Hover".to_string();
            image.color = ORANGE.into();
        }
        (false, false) => {
            **text = "Button".to_string();
            image.color = Color::WHITE;
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture_handle = asset_server.load("textures/fantasy_ui_borders/border_sheet.png");
    let atlas_layout =
        TextureAtlasLayout::from_grid(UVec2::new(50, 50), 6, 6, Some(UVec2::splat(2)), None);
    let atlas_layout_handle = texture_atlases.add(atlas_layout);

    let slicer = TextureSlicer {
        border: BorderRect::all(24.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };
    // ui camera
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            for (idx, [w, h]) in [
                (0, [150.0, 150.0]),
                (7, [300.0, 150.0]),
                (13, [150.0, 300.0]),
            ] {
                parent
                    .spawn((
                        Button,
                        Hovered::default(),
                        ImageNode::from_atlas_image(
                            texture_handle.clone(),
                            TextureAtlas {
                                index: idx,
                                layout: atlas_layout_handle.clone(),
                            },
                        )
                        .with_mode(NodeImageMode::Sliced(slicer.clone())),
                        Node {
                            width: px(w),
                            height: px(h),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            margin: px(20).all(),
                            ..default()
                        },
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new("Button"),
                            TextFont {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                                font_size: FontSize::Px(33.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ));
                    });
            }
        });
}
