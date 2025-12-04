//! Debug visualization for machines using gizmos.

use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_gizmos::prelude::*;
use bevy_math::Vec3;
use bevy_transform::components::Transform;

use super::components::{Machine, MachineType, WorkEnvelope};
use crate::config::EarthworksConfig;

/// System that draws work envelopes for all machines when debug is enabled.
pub fn draw_work_envelopes(
    config: Res<EarthworksConfig>,
    machines: Query<(&Machine, &WorkEnvelope, &Transform)>,
    mut gizmos: Gizmos,
) {
    if !config.show_work_envelopes {
        return;
    }

    for (machine, envelope, transform) in machines.iter() {
        let color = machine_color(machine.machine_type);
        let pos = transform.translation;
        let rotation = transform.rotation;

        match envelope {
            WorkEnvelope::Toroidal {
                inner_radius,
                outer_radius,
                min_height,
                max_height,
            } => {
                // Draw inner and outer circles at different heights
                for height in [*min_height, 0.0, *max_height] {
                    let center = pos + Vec3::Y * height;
                    draw_circle_xz(&mut gizmos, center, *inner_radius, color, 32);
                    draw_circle_xz(&mut gizmos, center, *outer_radius, color, 32);
                }

                // Draw vertical lines connecting the circles
                for i in 0..8 {
                    let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
                    let dir = Vec3::new(angle.cos(), 0.0, angle.sin());

                    let inner_bottom = pos + dir * *inner_radius + Vec3::Y * *min_height;
                    let inner_top = pos + dir * *inner_radius + Vec3::Y * *max_height;
                    gizmos.line(inner_bottom, inner_top, color);

                    let outer_bottom = pos + dir * *outer_radius + Vec3::Y * *min_height;
                    let outer_top = pos + dir * *outer_radius + Vec3::Y * *max_height;
                    gizmos.line(outer_bottom, outer_top, color);
                }
            }
            WorkEnvelope::Rectangular {
                width,
                depth,
                height,
            } => {
                // Transform rectangle corners to world space
                let half_width = *width / 2.0;
                let corners = [
                    Vec3::new(-half_width, 0.0, 0.0),
                    Vec3::new(half_width, 0.0, 0.0),
                    Vec3::new(half_width, 0.0, *depth),
                    Vec3::new(-half_width, 0.0, *depth),
                ];

                // Draw bottom and top rectangles
                for h in [0.0, *height] {
                    for i in 0..4 {
                        let a = pos + rotation * (corners[i] + Vec3::Y * h);
                        let b = pos + rotation * (corners[(i + 1) % 4] + Vec3::Y * h);
                        gizmos.line(a, b, color);
                    }
                }

                // Draw vertical edges
                for corner in &corners {
                    let bottom = pos + rotation * *corner;
                    let top = pos + rotation * (*corner + Vec3::Y * *height);
                    gizmos.line(bottom, top, color);
                }
            }
            WorkEnvelope::Arc {
                radius,
                angle,
                min_height,
                max_height,
            } => {
                let segments = 16;
                let half_angle = *angle / 2.0;

                for height in [*min_height, *max_height] {
                    let center = pos + Vec3::Y * height;

                    // Draw arc
                    for i in 0..segments {
                        let a1 = -half_angle + (i as f32 / segments as f32) * *angle;
                        let a2 = -half_angle + ((i + 1) as f32 / segments as f32) * *angle;

                        let p1 = center
                            + rotation * Vec3::new(a1.sin() * *radius, 0.0, a1.cos() * *radius);
                        let p2 = center
                            + rotation * Vec3::new(a2.sin() * *radius, 0.0, a2.cos() * *radius);
                        gizmos.line(p1, p2, color);
                    }

                    // Draw lines from center to arc ends
                    let left = center
                        + rotation
                            * Vec3::new(
                                (-half_angle).sin() * *radius,
                                0.0,
                                (-half_angle).cos() * *radius,
                            );
                    let right = center
                        + rotation
                            * Vec3::new(
                                half_angle.sin() * *radius,
                                0.0,
                                half_angle.cos() * *radius,
                            );
                    gizmos.line(center, left, color);
                    gizmos.line(center, right, color);
                }

                // Draw vertical lines at arc ends
                let left_dir = rotation * Vec3::new((-half_angle).sin(), 0.0, (-half_angle).cos());
                let right_dir = rotation * Vec3::new(half_angle.sin(), 0.0, half_angle.cos());

                gizmos.line(
                    pos + left_dir * *radius + Vec3::Y * *min_height,
                    pos + left_dir * *radius + Vec3::Y * *max_height,
                    color,
                );
                gizmos.line(
                    pos + right_dir * *radius + Vec3::Y * *min_height,
                    pos + right_dir * *radius + Vec3::Y * *max_height,
                    color,
                );
            }
        }
    }
}

/// Returns a color for a machine type.
fn machine_color(machine_type: MachineType) -> Color {
    match machine_type {
        MachineType::Excavator => Color::srgb(1.0, 0.8, 0.0), // Yellow
        MachineType::Dozer => Color::srgb(1.0, 0.5, 0.0),     // Orange
        MachineType::Loader => Color::srgb(0.0, 0.8, 0.2),    // Green
        MachineType::DumpTruck => Color::srgb(0.2, 0.6, 1.0), // Blue
    }
}

/// Helper to draw a circle in the XZ plane.
fn draw_circle_xz(gizmos: &mut Gizmos, center: Vec3, radius: f32, color: Color, segments: u32) {
    for i in 0..segments {
        let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let p1 = center + Vec3::new(a1.cos() * radius, 0.0, a1.sin() * radius);
        let p2 = center + Vec3::new(a2.cos() * radius, 0.0, a2.sin() * radius);
        gizmos.line(p1, p2, color);
    }
}
