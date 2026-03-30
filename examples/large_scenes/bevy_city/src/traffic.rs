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
}

impl TrafficLight {
    pub fn new(time_offset: f32) -> Self {
        Self { time_offset }
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

// Component/material for bulb
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLightColor {
    Red,
    Yellow,
    Green,
}

#[derive(Component)]
pub struct TrafficLightBulb {
    pub color: TrafficLightColor,
    pub light_entity: Entity,
    pub last_phase: Option<TrafficLightPhase>,
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
    traffic_lights: Query<(), With<TrafficLight>>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut commands: Commands,
) {
    let root = trigger.entity();
    if traffic_lights.get(root).is_err() {
        return;
    }
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
        commands.entity(descendant).insert(TrafficLightBulb {
            color,
            light_entity: root,
            last_phase: None,
        });
    }
}

fn update_bulb_materials(
    mut bulbs: Query<(Entity, &mut TrafficLightBulb)>,
    traffic_lights: Query<&TrafficLight>,
    time: Res<Time>,
    materials: Res<TrafficLightMaterials>,
    mut commands: Commands,
) {
    let elapsed = time.elapsed_secs();
    for (entity, mut bulb) in bulbs.iter_mut() {
        let Ok(light) = traffic_lights.get(bulb.light_entity) else {
            continue;
        };
        let phase = light.phase(elapsed);
        if bulb.last_phase == Some(phase) {
            continue;
        }
        bulb.last_phase = Some(phase);
        let handle = match (bulb.color, phase) {
            (TrafficLightColor::Red, TrafficLightPhase::Red) => &materials.red_on,
            (TrafficLightColor::Red, _) => &materials.red_off,
            (TrafficLightColor::Yellow, TrafficLightPhase::Yellow) => &materials.yellow_on,
            (TrafficLightColor::Yellow, _) => &materials.yellow_off,
            (TrafficLightColor::Green, TrafficLightPhase::Green) => &materials.green_on,
            (TrafficLightColor::Green, _) => &materials.green_off,
        };
        commands
            .entity(entity)
            .insert(MeshMaterial3d(handle.clone()));
    }
}
