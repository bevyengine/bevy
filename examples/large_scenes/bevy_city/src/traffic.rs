use bevy::{prelude::*, scene::SceneInstanceReady};

pub const RED_DURATION: f32 = 5.0;
pub const YELLOW_DURATION: f32 = 2.0;
pub const GREEN_DURATION: f32 = 5.0;

// Timing phase logic for traffic light
pub enum TrafficLightPhase {
    Red,
    Yellow,
    Green,
}

pub struct TrafficLight {
    pub time_offset: f32,
}


impl TrafficLight {
    pub fn new(time_offset: f32) -> Self {
        Self { time_offset }
    }

    pub fn phase(&self, time: f32) -> TrafficLightPhase {
        let cycle_time = RED_DURATION + YELLOW_DURATION + GREEN_DURATION;
        let time_in_cycle = (time + self.time_offset) % cycle_time;

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
pub enum TrafficLightColor {
    Red,
    Yellow,
    Green,
}

#[define(Component)]
pub struct TrafficLightBulb {
    pub color: TrafficLightColor,
    pub light_entity: Entity,
    pub last_phase: Option<TrafficLightPhase>,
}


// should it be red_on, yellow_on, etc...?
#[define(Resource)]
pub struct TrafficLightMaterials {
    pub red: Handle<StandardMaterial>,
    pub yellow: Handle<StandardMaterial>,
    pub green: Handle<StandardMaterial>,

}

pub struct TrafficPlugin;

impl Plugin for TrafficPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::update_traffic_lights);
    }
}

fn setup_traffic_light_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let red_material = materials.add(StandardMaterial {
        base_color: Color::rgb(1.0, 0.0, 0.0),
        emissive: Color::rgb(1.0, 0.0, 0.0),
        ..Default::default()
    });
    let yellow_material = materials.add(StandardMaterial {
        base_color: Color::rgb(1.0, 1.0, 0.0),
        emissive: Color::rgb(1.0, 1.0, 0.0),
        ..Default::default()
    });
    let green_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.0, 1.0, 0.0),
        emissive: Color::rgb(0.0, 1.0, 0.0),
        ..Default::default()
    });

    commands.insert_resource(TrafficLightMaterials {
        red: red_material,
        yellow: yellow_material,
        green: green_material,
    });


}

fn on_traffic_light_scene_ready(
    mut commands: Commands,
    mut ready_events: EventReader<SceneInstanceReady>,
    materials: Res<TrafficLightMaterials>,
) {
    for event in ready_events.iter() {
        if event.scene_handle == "traffic_light_scene_handle" {
            let traffic_light_entity = event.instance_id;
            let red_bulb_entity = commands.spawn().insert(TrafficLightBulb {
                color: TrafficLightColor::Red,
                light_entity: traffic_light_entity,
                last_phase: None,
            }).id();
            let yellow_bulb_entity = commands.spawn().insert(TrafficLightBulb {
                color: TrafficLightColor::Yellow,
                light_entity: traffic_light_entity,
                last_phase: None,
            }).id();
            let green_bulb_entity = commands.spawn().insert(TrafficLightBulb {
                color: TrafficLightColor::Green,
                light_entity: traffic_light_entity,
                last_phase: None,
            }).id();

            commands.entity(red_bulb_entity).insert(materials.red.clone());
            commands.entity(yellow_bulb_entity).insert(materials.yellow.clone());
            commands.entity(green_bulb_entity).insert(materials.green.clone());
        }
    }
}

fn update_bulb_materials(mut bulbs: Query<(Entity, &mut TrafficLightBulbMaterial)>, traffic_lights: Query<&TrafficLight>, time: Res<Time>, materials: Res<TrafficLightMaterial>,
commands: &mut Commands) {

    let elapsed = time.elapsed_secs();
    for (entity, mut bulb) in &mut bulbs {
        let traffic_light = traffic_lights.get(bulb.light_entity).unwrap();
        let phase = traffic_light.phase(elapsed);
        if Some(phase) != bulb.last_phase {
            let material_handle = match (bulb.color, phase) {

                
            }
        }
    }
}
