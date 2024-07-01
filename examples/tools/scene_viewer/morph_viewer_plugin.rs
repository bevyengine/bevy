//! Enable controls for morph targets detected in a loaded scene.
//!
//! Collect morph targets and assign keys to them,
//! shows on screen additional controls for morph targets.
//!
//! Illustrates how to access and modify individual morph target weights.
//! See the [`update_morphs`] system for details.
//!
//! Also illustrates how to read morph target names in [`detect_morphs`].

use crate::scene_viewer_plugin::SceneHandle;
use bevy::prelude::*;
use std::fmt;

const WEIGHT_PER_SECOND: f32 = 0.8;
const ALL_MODIFIERS: &[KeyCode] = &[KeyCode::ShiftLeft, KeyCode::ControlLeft, KeyCode::AltLeft];
const AVAILABLE_KEYS: [MorphKey; 56] = [
    MorphKey::new("r", &[], KeyCode::KeyR),
    MorphKey::new("t", &[], KeyCode::KeyT),
    MorphKey::new("z", &[], KeyCode::KeyZ),
    MorphKey::new("i", &[], KeyCode::KeyI),
    MorphKey::new("o", &[], KeyCode::KeyO),
    MorphKey::new("p", &[], KeyCode::KeyP),
    MorphKey::new("f", &[], KeyCode::KeyF),
    MorphKey::new("g", &[], KeyCode::KeyG),
    MorphKey::new("h", &[], KeyCode::KeyH),
    MorphKey::new("j", &[], KeyCode::KeyJ),
    MorphKey::new("k", &[], KeyCode::KeyK),
    MorphKey::new("y", &[], KeyCode::KeyY),
    MorphKey::new("x", &[], KeyCode::KeyX),
    MorphKey::new("c", &[], KeyCode::KeyC),
    MorphKey::new("v", &[], KeyCode::KeyV),
    MorphKey::new("b", &[], KeyCode::KeyB),
    MorphKey::new("n", &[], KeyCode::KeyN),
    MorphKey::new("m", &[], KeyCode::KeyM),
    MorphKey::new("0", &[], KeyCode::Digit0),
    MorphKey::new("1", &[], KeyCode::Digit1),
    MorphKey::new("2", &[], KeyCode::Digit2),
    MorphKey::new("3", &[], KeyCode::Digit3),
    MorphKey::new("4", &[], KeyCode::Digit4),
    MorphKey::new("5", &[], KeyCode::Digit5),
    MorphKey::new("6", &[], KeyCode::Digit6),
    MorphKey::new("7", &[], KeyCode::Digit7),
    MorphKey::new("8", &[], KeyCode::Digit8),
    MorphKey::new("9", &[], KeyCode::Digit9),
    MorphKey::new("lshift-R", &[KeyCode::ShiftLeft], KeyCode::KeyR),
    MorphKey::new("lshift-T", &[KeyCode::ShiftLeft], KeyCode::KeyT),
    MorphKey::new("lshift-Z", &[KeyCode::ShiftLeft], KeyCode::KeyZ),
    MorphKey::new("lshift-I", &[KeyCode::ShiftLeft], KeyCode::KeyI),
    MorphKey::new("lshift-O", &[KeyCode::ShiftLeft], KeyCode::KeyO),
    MorphKey::new("lshift-P", &[KeyCode::ShiftLeft], KeyCode::KeyP),
    MorphKey::new("lshift-F", &[KeyCode::ShiftLeft], KeyCode::KeyF),
    MorphKey::new("lshift-G", &[KeyCode::ShiftLeft], KeyCode::KeyG),
    MorphKey::new("lshift-H", &[KeyCode::ShiftLeft], KeyCode::KeyH),
    MorphKey::new("lshift-J", &[KeyCode::ShiftLeft], KeyCode::KeyJ),
    MorphKey::new("lshift-K", &[KeyCode::ShiftLeft], KeyCode::KeyK),
    MorphKey::new("lshift-Y", &[KeyCode::ShiftLeft], KeyCode::KeyY),
    MorphKey::new("lshift-X", &[KeyCode::ShiftLeft], KeyCode::KeyX),
    MorphKey::new("lshift-C", &[KeyCode::ShiftLeft], KeyCode::KeyC),
    MorphKey::new("lshift-V", &[KeyCode::ShiftLeft], KeyCode::KeyV),
    MorphKey::new("lshift-B", &[KeyCode::ShiftLeft], KeyCode::KeyB),
    MorphKey::new("lshift-N", &[KeyCode::ShiftLeft], KeyCode::KeyN),
    MorphKey::new("lshift-M", &[KeyCode::ShiftLeft], KeyCode::KeyM),
    MorphKey::new("lshift-0", &[KeyCode::ShiftLeft], KeyCode::Digit0),
    MorphKey::new("lshift-1", &[KeyCode::ShiftLeft], KeyCode::Digit1),
    MorphKey::new("lshift-2", &[KeyCode::ShiftLeft], KeyCode::Digit2),
    MorphKey::new("lshift-3", &[KeyCode::ShiftLeft], KeyCode::Digit3),
    MorphKey::new("lshift-4", &[KeyCode::ShiftLeft], KeyCode::Digit4),
    MorphKey::new("lshift-5", &[KeyCode::ShiftLeft], KeyCode::Digit5),
    MorphKey::new("lshift-6", &[KeyCode::ShiftLeft], KeyCode::Digit6),
    MorphKey::new("lshift-7", &[KeyCode::ShiftLeft], KeyCode::Digit7),
    MorphKey::new("lshift-8", &[KeyCode::ShiftLeft], KeyCode::Digit8),
    MorphKey::new("lshift-9", &[KeyCode::ShiftLeft], KeyCode::Digit9),
];

#[derive(Clone, Copy)]
enum WeightChange {
    Increase,
    Decrease,
}
impl WeightChange {
    fn reverse(&mut self) {
        *self = match *self {
            WeightChange::Increase => WeightChange::Decrease,
            WeightChange::Decrease => WeightChange::Increase,
        }
    }
    fn sign(self) -> f32 {
        match self {
            WeightChange::Increase => 1.0,
            WeightChange::Decrease => -1.0,
        }
    }
    fn change_weight(&mut self, weight: f32, change: f32) -> f32 {
        let mut change = change * self.sign();
        let new_weight = weight + change;
        if new_weight <= 0.0 || new_weight >= 1.0 {
            self.reverse();
            change = -change;
        }
        weight + change
    }
}

struct Target {
    entity_name: Option<String>,
    entity: Entity,
    name: Option<String>,
    index: usize,
    weight: f32,
    change_dir: WeightChange,
}
impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.name.as_ref(), self.entity_name.as_ref()) {
            (None, None) => write!(f, "animation{} of {:?}", self.index, self.entity),
            (None, Some(entity)) => write!(f, "animation{} of {entity}", self.index),
            (Some(target), None) => write!(f, "{target} of {:?}", self.entity),
            (Some(target), Some(entity)) => write!(f, "{target} of {entity}"),
        }?;
        write!(f, ": {}", self.weight)
    }
}
impl Target {
    fn text_section(&self, key: &str, style: TextStyle) -> TextSection {
        TextSection::new(format!("[{key}] {self}\n"), style)
    }
    fn new(
        entity_name: Option<&Name>,
        weights: &[f32],
        target_names: Option<&[String]>,
        entity: Entity,
    ) -> Vec<Target> {
        let get_name = |i| target_names.and_then(|names| names.get(i));
        let entity_name = entity_name.map(|n| n.as_str());
        weights
            .iter()
            .enumerate()
            .map(|(index, weight)| Target {
                entity_name: entity_name.map(|n| n.to_owned()),
                entity,
                name: get_name(index).cloned(),
                index,
                weight: *weight,
                change_dir: WeightChange::Increase,
            })
            .collect()
    }
}

#[derive(Resource)]
struct WeightsControl {
    weights: Vec<Target>,
}

struct MorphKey {
    name: &'static str,
    modifiers: &'static [KeyCode],
    key: KeyCode,
}
impl MorphKey {
    const fn new(name: &'static str, modifiers: &'static [KeyCode], key: KeyCode) -> Self {
        MorphKey {
            name,
            modifiers,
            key,
        }
    }
    fn active(&self, inputs: &ButtonInput<KeyCode>) -> bool {
        let mut modifier = self.modifiers.iter();
        let mut non_modifier = ALL_MODIFIERS.iter().filter(|m| !self.modifiers.contains(m));

        let key = inputs.pressed(self.key);
        let modifier = modifier.all(|m| inputs.pressed(*m));
        let non_modifier = non_modifier.all(|m| !inputs.pressed(*m));
        key && modifier && non_modifier
    }
}
fn update_text(
    controls: Option<ResMut<WeightsControl>>,
    mut text: Query<&mut Text>,
    morphs: Query<&MorphWeights>,
) {
    let Some(mut controls) = controls else {
        return;
    };
    for (i, target) in controls.weights.iter_mut().enumerate() {
        let Ok(weights) = morphs.get(target.entity) else {
            continue;
        };
        let Some(&actual_weight) = weights.weights().get(target.index) else {
            continue;
        };
        if actual_weight != target.weight {
            target.weight = actual_weight;
        }
        let key_name = &AVAILABLE_KEYS[i].name;
        let mut text = text.single_mut();
        text.sections[i + 2].value = format!("[{key_name}] {target}\n");
    }
}
fn update_morphs(
    controls: Option<ResMut<WeightsControl>>,
    mut morphs: Query<&mut MorphWeights>,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Some(mut controls) = controls else {
        return;
    };
    for (i, target) in controls.weights.iter_mut().enumerate() {
        if !AVAILABLE_KEYS[i].active(&input) {
            continue;
        }
        let Ok(mut weights) = morphs.get_mut(target.entity) else {
            continue;
        };
        // To update individual morph target weights, get the `MorphWeights`
        // component and call `weights_mut` to get access to the weights.
        let weights_slice = weights.weights_mut();
        let i = target.index;
        let change = time.delta_seconds() * WEIGHT_PER_SECOND;
        let new_weight = target.change_dir.change_weight(weights_slice[i], change);
        weights_slice[i] = new_weight;
        target.weight = new_weight;
    }
}

fn detect_morphs(
    mut commands: Commands,
    morphs: Query<(Entity, &MorphWeights, Option<&Name>)>,
    meshes: Res<Assets<Mesh>>,
    scene_handle: Res<SceneHandle>,
    mut setup: Local<bool>,
) {
    let no_morphing = morphs.iter().len() == 0;
    if no_morphing {
        return;
    }
    if scene_handle.is_loaded && !*setup {
        *setup = true;
    } else {
        return;
    }
    let mut detected = Vec::new();

    for (entity, weights, name) in &morphs {
        let target_names = weights
            .first_mesh()
            .and_then(|h| meshes.get(h))
            .and_then(|m| m.morph_target_names());
        let targets = Target::new(name, weights.weights(), target_names, entity);
        detected.extend(targets);
    }
    detected.truncate(AVAILABLE_KEYS.len());
    let style = TextStyle {
        font_size: 13.0,
        ..default()
    };
    let mut sections = vec![
        TextSection::new("Morph Target Controls\n", style.clone()),
        TextSection::new("---------------\n", style.clone()),
    ];
    let target_to_text =
        |(i, target): (usize, &Target)| target.text_section(AVAILABLE_KEYS[i].name, style.clone());
    sections.extend(detected.iter().enumerate().map(target_to_text));
    commands.insert_resource(WeightsControl { weights: detected });
    commands.spawn(TextBundle::from_sections(sections).with_style(Style {
        position_type: PositionType::Absolute,
        top: Val::Px(10.0),
        left: Val::Px(10.0),
        ..default()
    }));
}

pub struct MorphViewerPlugin;

impl Plugin for MorphViewerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_morphs,
                detect_morphs,
                update_text.after(update_morphs),
            ),
        );
    }
}
