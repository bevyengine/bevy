#define_import_path example::shared_group::common

struct Time {
    seconds_since_startup: f32,
};

struct Emitter {
    position: vec3<f32>,
    radius: f32,
    strength: f32,
    propagation_speed: f32,
    phase_speed: f32,
};

@group(3) @binding(0)
var<uniform> time: Time;
@group(3) @binding(1)
var<uniform> emitter: Emitter;

fn field_phase(distance: f32) -> f32 {
    return sin((time.seconds_since_startup * emitter.propagation_speed) - (distance * emitter.phase_speed));
}

fn field_amplitude(distance: f32) -> f32 {
    let amp = (emitter.strength * (emitter.radius - distance))/(emitter.radius * (distance + 1.0));
    return max(0.0, amp);
}

fn field_impact(position: vec3<f32>) -> f32 {
    let dist = distance(position, emitter.position);
    return field_amplitude(dist) * (0.5 + field_phase(dist) * 0.5);
}
