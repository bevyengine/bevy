use bevy::{prelude::*, text::FontAtlasSet};

fn main() {
    App::build()
        .init_resource::<State>()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(text_update_system.system())
        .add_system(atlas_render_system.system())
        .run();
}

struct State {
    added: bool,
    handle: Handle<Font>,
    timer: Timer,
}

impl Default for State {
    fn default() -> Self {
        Self {
            added: false,
            handle: Handle::default(),
            timer: Timer::from_seconds(0.05),
        }
    }
}

fn atlas_render_system(
    command_buffer: &mut CommandBuffer,
    mut state: ResMut<State>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    font_atlas_sets: Res<Assets<FontAtlasSet>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    if state.added {
        return;
    }
    if let Some(set) = font_atlas_sets.get(&state.handle.as_handle::<FontAtlasSet>()) {
        for (_size, font_atlas) in set.iter() {
            state.added = true;
            let texture_atlas = texture_atlases.get(&font_atlas.texture_atlas).unwrap();
            command_buffer.build().add_entity(SpriteEntity {
                material: materials.add(texture_atlas.texture.into()),
                translation: Vec3::new(-300.0, 0., 0.0).into(),
                ..Default::default()
            });
            break;
        }
    }
}

fn text_update_system(mut state: ResMut<State>, time: Res<Time>, mut label: ComMut<Label>) {
    state.timer.tick(time.delta_seconds);
    if state.timer.finished {
        label.text = format!("{}", rand::random::<u8>() as char);
        state.timer.reset();
    }
}

fn setup(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut state: ResMut<State>,
) {
    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    state.handle = font_handle;
    command_buffer
        .build()
        // 2d camera
        .add_entity(OrthographicCameraEntity::default())
        .add_entity(OrthographicCameraEntity::ui())
        // texture
        .add_entity(LabelEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::TOP_LEFT,
                Margins::new(0.0, 250.0, 0.0, 60.0),
            ),
            label: Label {
                text: "a".to_string(),
                font: font_handle,
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        });
}
