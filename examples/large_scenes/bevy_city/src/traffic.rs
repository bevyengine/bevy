use crate::settings::Settings;
use bevy::{prelude::*, scene::SceneInstanceReady};

pub const RED_DURATION: f32 = 5.0;
pub const YELLOW_DURATION: f32 = 2.0;
pub const GREEN_DURATION: f32 = 5.0;
pub const CYCLE_DURATION: f32 = RED_DURATION + YELLOW_DURATION + GREEN_DURATION;

// Timing phase logic for traffic light
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLightPhase {
    Red,
    Yellow,
    Green,
}

#[derive(Component)]
pub struct TrafficLight {
    pub time_offset: f32,
    last_phase: Option<TrafficLightPhase>,
    bulbs: Vec<(Entity, TrafficLightColor)>,
}

impl TrafficLight {
    pub fn new(time_offset: f32) -> Self {
        Self {
            time_offset,
            last_phase: None,
            bulbs: Vec::new(),
        }
    }

    pub fn phase(&self, time: f32) -> TrafficLightPhase {
        let time_in_cycle = (time + self.time_offset).rem_euclid(CYCLE_DURATION);

        if time_in_cycle < RED_DURATION {
            TrafficLightPhase::Red
        } else if time_in_cycle < RED_DURATION + YELLOW_DURATION {
            TrafficLightPhase::Yellow
        } else {
            TrafficLightPhase::Green
        }
    }
}

// Component for each bulb on a traffic light, which is updated based on light's current phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLightColor {
    Red,
    Yellow,
    Green,
}

#[derive(Resource)]
pub struct TrafficLightMaterials {
    pub red_on: Handle<StandardMaterial>,
    pub red_off: Handle<StandardMaterial>,
    pub yellow_on: Handle<StandardMaterial>,
    pub yellow_off: Handle<StandardMaterial>,
    pub green_on: Handle<StandardMaterial>,
    pub green_off: Handle<StandardMaterial>,
}

pub struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_traffic_light_materials)
            .add_systems(Update, update_bulb_materials)
            .add_observer(on_traffic_light_scene_ready);
    }
}

fn setup_traffic_light_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TrafficLightMaterials {
        red_on: materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.0, 0.0),
            emissive: LinearRgba::new(12.0, 0.0, 0.0, 1.0),
            ..default()
        }),
        red_off: materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.02, 0.02),
            ..default()
        }),
        yellow_on: materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.3, 0.0),
            emissive: LinearRgba::new(10.0, 6.0, 0.0, 1.0),
            ..default()
        }),
        yellow_off: materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.12, 0.02),
            ..default()
        }),
        green_on: materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.4, 0.0),
            emissive: LinearRgba::new(0.0, 12.0, 0.0, 1.0),
            ..default()
        }),
        green_off: materials.add(StandardMaterial {
            base_color: Color::srgb(0.02, 0.15, 0.02),
            ..default()
        }),
    });
}

fn on_traffic_light_scene_ready(
    trigger: On<SceneInstanceReady>,
    mut traffic_lights: Query<&mut TrafficLight>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut mesh_materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    light_materials: Res<TrafficLightMaterials>,
) {
    let root = trigger.entity;
    let Ok(mut light) = traffic_lights.get_mut(root) else {
        return;
    };
    for descendant in children.iter_descendants(root) {
        let Ok(name) = names.get(descendant) else {
            continue;
        };
        let color = match name.as_str() {
            "RED_LIGHT" => TrafficLightColor::Red,
            "YELLOW_LIGHT" => TrafficLightColor::Yellow,
            "GREEN_LIGHT" => TrafficLightColor::Green,
            _ => continue,
        };
        light.bulbs.push((descendant, color));
        if let Ok(mut material) = mesh_materials.get_mut(descendant) {
            material.0 = light_materials.red_off.clone();
        }
    }
}

fn update_bulb_materials(
    settings: Res<Settings>,
    mut traffic_lights: Query<&mut TrafficLight>,
    mut mesh_materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    time: Res<Time>,
    materials: Res<TrafficLightMaterials>,
) {
    if !settings.traffic_enabled {
        return;
    }
    let elapsed = time.elapsed_secs();
    for mut light in traffic_lights.iter_mut() {
        let phase = light.phase(elapsed);
        if light.last_phase == Some(phase) {
            continue;
        }
        light.last_phase = Some(phase);
        for (entity, color) in &light.bulbs {
            let Ok(mut material) = mesh_materials.get_mut(*entity) else {
                continue;
            };
            material.0 = match (color, phase) {
                (TrafficLightColor::Red, TrafficLightPhase::Red) => materials.red_on.clone(),
                (TrafficLightColor::Red, _) => materials.red_off.clone(),
                (TrafficLightColor::Yellow, TrafficLightPhase::Yellow) => {
                    materials.yellow_on.clone()
                }
                (TrafficLightColor::Yellow, _) => materials.yellow_off.clone(),
                (TrafficLightColor::Green, TrafficLightPhase::Green) => materials.green_on.clone(),
                (TrafficLightColor::Green, _) => materials.green_off.clone(),
            };
        }
    }
}
