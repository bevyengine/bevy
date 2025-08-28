//! basic bevy 2d text input
use std::any::TypeId;

use bevy::{
    camera::{primitives::Aabb, visibility::VisibilityClass},
    color::palettes::{
        css::NAVY,
        tailwind::{BLUE_900, GRAY_300, GRAY_400, SKY_300},
    },
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    render::{sync_world::TemporaryRenderEntity, Extract, RenderApp},
    sprite::Anchor,
    sprite_render::{
        ExtractedSlice, ExtractedSlices, ExtractedSprite, ExtractedSpriteKind, ExtractedSprites,
    },
    text::{
        LineBreak, Motion, Placeholder, PlaceholderLayout, PositionedGlyph, TextBounds,
        TextCursorBlinkInterval, TextEdit, TextEdits, TextInputAttributes, TextInputBuffer,
        TextInputEvent, TextInputSystems, TextInputTarget, TextLayoutInfo,
    },
    window::PrimaryWindow,
};

#[derive(Component)]
struct TextInputSize(Vec2);

#[derive(Component, Default)]
struct Overwrite(bool);

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            (update_inputs, update_targets).before(TextInputSystems),
        );
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, extract_text_input);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let submit_target = commands
        .spawn((
            Text2d::new("submit with SHIFT + ENTER"),
            Anchor::TOP_CENTER,
            TextBounds {
                width: Some(500.),
                height: None,
            },
            TextLayout {
                linebreak: LineBreak::AnyCharacter,
                justify: Justify::Left,
            },
            Transform::from_translation(Vec3::new(0., -25., 0.)),
        ))
        .id();

    commands
        .spawn((
            TextInputBuffer {
                cursor_blink_timer: Some(0.),
                ..Default::default()
            },
            TextInputAttributes::default(),
            Overwrite::default(),
            TextInputSize(Vec2::new(500., 250.)),
            Transform::from_translation(Vec3::new(0., 150., 0.)),
            Placeholder::new("type here.."),
            Visibility::default(),
            VisibilityClass([TypeId::of::<Sprite>()].into()),
            Anchor::CENTER,
        ))
        .observe(
            move |event: On<TextInputEvent>, mut query: Query<&mut Text2d>| {
                if let TextInputEvent::Submission { text } = event.event()
                    && let Ok(mut target) = query.get_mut(submit_target)
                {
                    target.0 = text.clone();
                }
            },
        );
}

fn update_targets(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(
        Entity,
        &TextInputSize,
        &mut TextInputTarget,
        &Anchor,
        Option<&mut Aabb>,
    )>,
) {
    let scale_factor = windows
        .single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);
    for (entity, size, mut target, anchor, aabb) in query.iter_mut() {
        if target.set_if_neq(TextInputTarget {
            size: size.0 * scale_factor,
            scale_factor,
        }) {
            let (x1, x2, y1, y2) = anchor.reposition(size.0);
            let new_aabb = Aabb::from_min_max(Vec3::new(x1, y1, 0.), Vec3::new(x2, y2, 0.));

            if let Some(mut aabb) = aabb {
                *aabb = new_aabb;
            } else {
                commands.entity(entity).try_insert(new_aabb);
            }
        }
    }
}

fn update_inputs(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut query: Query<(&mut TextEdits, &mut Overwrite)>,
    keyboard_state: Res<ButtonInput<Key>>,
) {
    for key_input in keyboard_events.read() {
        if !key_input.state.is_pressed() {
            return;
        }

        let pressed_key = &key_input.logical_key;
        let is_shift_pressed = keyboard_state.pressed(Key::Shift);
        #[cfg(not(target_os = "macos"))]
        let is_command_pressed = keyboard_state.pressed(Key::Control);
        #[cfg(target_os = "macos")]
        let is_command_pressed = keyboard_state.pressed(Key::Super);

        for (mut actions, mut overwrite) in query.iter_mut() {
            if is_command_pressed {
                match &pressed_key {
                    Key::Character(str) => {
                        if let Some(char) = str.chars().next() {
                            // convert to lowercase so that the commands work with capslock on
                            match (char.to_ascii_lowercase(), is_shift_pressed) {
                                ('c', false) => {
                                    // copy
                                    actions.queue(TextEdit::Copy);
                                }
                                ('x', false) => {
                                    // cut
                                    actions.queue(TextEdit::Cut);
                                }
                                ('v', false) => {
                                    // paste
                                    actions.queue(TextEdit::Paste);
                                }
                                ('a', false) => {
                                    // select all
                                    actions.queue(TextEdit::SelectAll);
                                }
                                _ => {
                                    // not recognized, ignore
                                }
                            }
                        }
                    }
                    Key::ArrowLeft => {
                        actions.queue(TextEdit::Motion {
                            motion: Motion::PreviousWord,
                            with_select: is_shift_pressed,
                        });
                    }
                    Key::ArrowRight => {
                        actions.queue(TextEdit::Motion {
                            motion: Motion::NextWord,
                            with_select: is_shift_pressed,
                        });
                    }
                    Key::ArrowUp => {
                        actions.queue(TextEdit::Scroll { lines: -1 });
                    }
                    Key::ArrowDown => {
                        actions.queue(TextEdit::Scroll { lines: 1 });
                    }
                    Key::Home => {
                        actions.queue(TextEdit::motion(Motion::BufferStart, is_shift_pressed));
                    }
                    Key::End => {
                        actions.queue(TextEdit::motion(Motion::BufferEnd, is_shift_pressed));
                    }
                    _ => {
                        // not recognized, ignore
                    }
                }
            } else {
                match &pressed_key {
                    Key::Character(_) | Key::Space => {
                        let str = if let Key::Character(str) = &pressed_key {
                            str.chars()
                        } else {
                            " ".chars()
                        };
                        for char in str {
                            actions.queue(if overwrite.0 {
                                TextEdit::Overwrite(char)
                            } else {
                                TextEdit::Insert(char)
                            });
                        }
                    }
                    Key::Enter => {
                        if is_shift_pressed {
                            actions.queue(TextEdit::Submit);
                        } else {
                            actions.queue(TextEdit::NewLine);
                        }
                    }
                    Key::Backspace => {
                        actions.queue(TextEdit::Backspace);
                    }
                    Key::Delete => {
                        if is_shift_pressed {
                            actions.queue(TextEdit::Cut);
                        } else {
                            actions.queue(TextEdit::Delete);
                        }
                    }
                    Key::PageUp => {
                        actions.queue(TextEdit::motion(Motion::PageUp, is_shift_pressed));
                    }
                    Key::PageDown => {
                        actions.queue(TextEdit::motion(Motion::PageDown, is_shift_pressed));
                    }
                    Key::ArrowLeft => {
                        actions.queue(TextEdit::motion(Motion::Left, is_shift_pressed));
                    }
                    Key::ArrowRight => {
                        actions.queue(TextEdit::motion(Motion::Right, is_shift_pressed));
                    }
                    Key::ArrowUp => {
                        actions.queue(TextEdit::motion(Motion::Up, is_shift_pressed));
                    }
                    Key::ArrowDown => {
                        actions.queue(TextEdit::motion(Motion::Down, is_shift_pressed));
                    }
                    Key::Home => {
                        actions.queue(TextEdit::motion(Motion::Home, is_shift_pressed));
                    }
                    Key::End => {
                        actions.queue(TextEdit::motion(Motion::End, is_shift_pressed));
                    }
                    Key::Escape => {
                        actions.queue(TextEdit::Escape);
                    }
                    Key::Tab => {
                        actions.queue(if is_shift_pressed {
                            TextEdit::Unindent
                        } else {
                            TextEdit::Indent
                        });
                    }
                    Key::Insert => {
                        overwrite.0 = !overwrite.0;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn extract_text_input(
    mut commands: Commands,
    mut extracted_sprites: ResMut<ExtractedSprites>,
    mut extracted_slices: ResMut<ExtractedSlices>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    cursor_blink_interval: Extract<Res<TextCursorBlinkInterval>>,
    text_input_query: Extract<
        Query<(
            Entity,
            &TextInputBuffer,
            &TextLayoutInfo,
            Option<&PlaceholderLayout>,
            &TextInputTarget,
            &Overwrite,
            &ViewVisibility,
            &Anchor,
            &GlobalTransform,
        )>,
    >,
) {
    let mut start = extracted_slices.slices.len();
    let mut end = start + 1;

    for (
        main_entity,
        buffer,
        text_layout,
        maybe_placeholder_layout,
        target,
        overwrite,
        view_visibility,
        anchor,
        global_transform,
    ) in text_input_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }

        let top_left = (Anchor::TOP_LEFT.as_vec() - anchor.as_vec()) * target.size;

        let (layout, color): (_, Color) = if let Some(placeholder_layout) =
            maybe_placeholder_layout.filter(|_| buffer.is_empty())
        {
            (placeholder_layout.layout(), SKY_300.into())
        } else {
            (text_layout, GRAY_300.into())
        };

        let transform = *global_transform
            * GlobalTransform::from_scale(Vec2::splat(target.scale_factor.recip()).extend(1.))
            * GlobalTransform::from_translation(top_left.extend(0.));

        extracted_sprites.sprites.push(ExtractedSprite {
            main_entity,
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            transform,
            color: NAVY.into(),
            image_handle_id: AssetId::default(),
            flip_x: false,
            flip_y: false,
            kind: ExtractedSpriteKind::Single {
                anchor: Anchor::TOP_LEFT.as_vec(),
                rect: None,
                scaling_mode: None,
                custom_size: Some(target.size),
            },
        });

        let blink = buffer
            .cursor_blink_timer
            .is_none_or(|t| cursor_blink_interval.0.as_secs_f32() < t);

        let transform = transform
            * GlobalTransform::from_translation(
                Vec2::new(-layout.scroll.x, layout.scroll.y).extend(0.),
            );

        for (i, rect) in layout.selection_rects.iter().enumerate() {
            let size = if (1..layout.selection_rects.len()).contains(&i) {
                rect.size() + Vec2::Y
            } else {
                rect.size()
            } + 2. * Vec2::X;

            extracted_sprites.sprites.push(ExtractedSprite {
                main_entity,
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                transform: transform
                    * GlobalTransform::from_translation(Vec3::new(rect.min.x, -rect.min.y, 0.)),
                color: BLUE_900.into(),
                image_handle_id: AssetId::default(),
                flip_x: false,
                flip_y: false,
                kind: ExtractedSpriteKind::Single {
                    anchor: Anchor::TOP_LEFT.as_vec(),
                    rect: None,
                    scaling_mode: None,
                    custom_size: Some(size),
                },
            });
        }

        for (
            i,
            PositionedGlyph {
                position,
                atlas_info,
                ..
            },
        ) in layout.glyphs.iter().enumerate()
        {
            let rect = texture_atlases
                .get(atlas_info.texture_atlas)
                .unwrap()
                .textures[atlas_info.location.glyph_index]
                .as_rect();
            extracted_slices.slices.push(ExtractedSlice {
                offset: Vec2::new(position.x, -position.y),
                rect,
                size: rect.size(),
            });

            if layout.glyphs.get(i + 1).is_none_or(|info| {
                info.atlas_info.texture != atlas_info.texture
                    || layout.cursor_index.is_some_and(|j| j == i + 1)
            }) {
                extracted_sprites.sprites.push(ExtractedSprite {
                    main_entity,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    transform,
                    color: color.into(),
                    image_handle_id: atlas_info.texture,
                    flip_x: false,
                    flip_y: false,
                    kind: ExtractedSpriteKind::Slices {
                        indices: start..end,
                    },
                });
                start = end;
            }
            end += 1;
        }

        if blink {
            continue;
        }

        let Some((position, layout_cursor_size, _affinity)) = layout.cursor else {
            continue;
        };

        let (w, _cursor_z_offset) = if overwrite.0 {
            (layout_cursor_size.x, -0.001)
        } else {
            (0.2 * buffer.space_advance, 0.)
        };

        let cursor_size = Vec2::new(w, layout_cursor_size.y).ceil();
        let cursor_x = position.x - 0.5 * (layout_cursor_size.x - cursor_size.x);
        let cursor_y = -position.y;

        extracted_sprites.sprites.push(ExtractedSprite {
            main_entity,
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            transform: transform
                * GlobalTransform::from_translation(Vec3::new(cursor_x, cursor_y, 0.)),
            color: GRAY_400.into(),
            image_handle_id: AssetId::default(),
            flip_x: false,
            flip_y: false,
            kind: ExtractedSpriteKind::Single {
                anchor: Vec2::ZERO,
                rect: None,
                scaling_mode: None,
                custom_size: Some(cursor_size),
            },
        });
    }
}
