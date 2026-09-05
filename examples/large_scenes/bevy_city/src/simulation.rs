use bevy::prelude::*;
use rand::{rngs::SmallRng, RngExt, SeedableRng};

use crate::{settings::Settings, Args};

#[derive(Component)]
pub struct Road;

#[derive(Component)]
pub struct Lane {
    pub start: Vec3,
    pub end: Vec3,
    pub target_node: Entity,
}

#[derive(Component)]
pub struct Intersection {
    pub lanes: Vec<Entity>,
}

pub const LANE_WIDTH: f32 = 0.15;

#[derive(Component)]
pub struct Car {
    /// The lane this car is currently driving on.
    pub lane: Entity,
    /// Distance traveled along the current lane.
    pub progress: f32,
}

pub fn simulate_cars(
    settings: Res<Settings>,
    args: Res<Args>,
    lanes: Query<&Lane, Without<Car>>,
    intersections: Query<&Intersection>,
    mut cars: Query<(&mut Car, &mut Transform), Without<Lane>>,
    time: Res<Time>,
    mut rng: Local<Option<SmallRng>>,
) {
    if !settings.simulate_cars {
        return;
    }
    let rng = rng.get_or_insert_with(|| SmallRng::seed_from_u64(args.seed));
    let speed = 1.5;

    for (mut car, mut car_transform) in &mut cars {
        let Ok(mut lane) = lanes.get(car.lane) else {
            continue;
        };

        car.progress += speed * time.delta_secs();
        let mut lane_len = (lane.end - lane.start).length();

        if car.progress > lane_len {
            let Ok(intersection) = intersections.get(lane.target_node) else {
                continue;
            };
            let next_lane_entity =
                intersection.lanes[rng.random_range(0..intersection.lanes.len())];
            let Ok(next_lane) = lanes.get(next_lane_entity) else {
                continue;
            };

            car.progress = 0.0;
            car.lane = next_lane_entity;

            lane = next_lane;
            lane_len = (lane.end - lane.start).length();
        }

        let progress = (car.progress / lane_len).clamp(0.0, 1.0);
        let direction = (lane.end - lane.start).normalize();
        let lane_offset = direction.cross(Vec3::Y) * LANE_WIDTH;
        car_transform.translation = lane.start.lerp(lane.end, progress) + lane_offset;
        car_transform.look_to(-direction, Vec3::Y);
    }
}
