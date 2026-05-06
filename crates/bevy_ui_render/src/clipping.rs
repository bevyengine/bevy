use bevy_math::{Affine2, Vec2};
use bevy_ui::CalculatedClip;
use smallvec::SmallVec;

/// Assumes `vertices` in boundary order forming a convex polygon.
/// Returns a list of vertices that form a triangle fan.
pub(crate) fn clip_polygon<T: Copy>(
    clip: Option<&CalculatedClip>,
    vertices: &[(Vec2, T)],
    interpolate: impl Fn(T, T, f32) -> T + Copy,
) -> SmallVec<[(Vec2, T); 16]> {
    // If less than 3 vertices, there's no visible region to clip.
    if vertices.len() < 3 {
        return SmallVec::new();
    }

    let Some(clip) = clip else {
        return SmallVec::from_slice(vertices);
    };
    let Some(rects) = clip.rects() else {
        return SmallVec::new();
    };

    let mut visible_region = SmallVec::from_slice(vertices);
    let mut scratch = SmallVec::new();

    for region in rects {
        if visible_region.len() < 3 {
            break;
        }

        if region.rect.min.x.is_finite() {
            edge_clip(
                &visible_region,
                &mut scratch,
                region.world_to_clip_local,
                |point| point.x - region.rect.min.x,
                interpolate,
            );
            core::mem::swap(&mut visible_region, &mut scratch);
        }
        if region.rect.max.x.is_finite() {
            edge_clip(
                &visible_region,
                &mut scratch,
                region.world_to_clip_local,
                |point| region.rect.max.x - point.x,
                interpolate,
            );
            core::mem::swap(&mut visible_region, &mut scratch);
        }
        if region.rect.min.y.is_finite() {
            edge_clip(
                &visible_region,
                &mut scratch,
                region.world_to_clip_local,
                |point| point.y - region.rect.min.y,
                interpolate,
            );
            core::mem::swap(&mut visible_region, &mut scratch);
        }
        if region.rect.max.y.is_finite() {
            edge_clip(
                &visible_region,
                &mut scratch,
                region.world_to_clip_local,
                |point| region.rect.max.y - point.y,
                interpolate,
            );
            core::mem::swap(&mut visible_region, &mut scratch);
        }
    }

    if visible_region.len() < 3 {
        visible_region.clear();
    }

    visible_region
}

fn edge_clip<T: Copy>(
    input: &[(Vec2, T)],
    output: &mut SmallVec<[(Vec2, T); 16]>,
    world_to_clip: Affine2,
    signed_distance: impl Fn(Vec2) -> f32,
    interpolate: impl Fn(T, T, f32) -> T + Copy,
) {
    output.clear();

    let Some(mut previous) = input.last().copied() else {
        return;
    };
    let mut previous_distance = signed_distance(world_to_clip.transform_point2(previous.0));
    let mut is_previous_visible = 0. <= previous_distance;

    for &vertex in input {
        let distance = signed_distance(world_to_clip.transform_point2(vertex.0));
        let is_visible = 0. <= distance;
        // If inside != previous_inside, the previous -> vertex edge crossed the clip rect edge and we
        // add a new vertex at the intersection.
        if is_visible != is_previous_visible {
            let t = previous_distance / (previous_distance - distance);
            output.push((
                previous.0.lerp(vertex.0, t),
                interpolate(previous.1, vertex.1, t),
            ));
        }
        if is_visible {
            output.push(vertex);
        }
        previous = vertex;
        previous_distance = distance;
        is_previous_visible = is_visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::{vec2, Affine2, Mat2, Rect, Rot2};
    use bevy_ui::{CalculatedClip, CalculatedClipRect};

    fn calculated_clip(regions: impl IntoIterator<Item = CalculatedClipRect>) -> CalculatedClip {
        CalculatedClip::Rects(regions.into_iter().collect())
    }

    fn quad() -> [(Vec2, Vec2); 4] {
        [
            (vec2(-1., -1.), vec2(-1., -1.)),
            (vec2(1., -1.), vec2(1., -1.)),
            (vec2(1., 1.), vec2(1., 1.)),
            (vec2(-1., 1.), vec2(-1., 1.)),
        ]
    }

    #[test]
    fn unclipped_quad_returns_all_vertices() {
        assert_eq!(clip_polygon(None, &quad(), Vec2::lerp).len(), 4);
    }

    #[test]
    fn fully_clipped_returns_empty_vertices_list() {
        assert!(clip_polygon(Some(&CalculatedClip::FullyClipped), &quad(), Vec2::lerp).is_empty());
    }

    #[test]
    fn trim_quad_with_axis_aligned_clip() {
        let clip = calculated_clip([CalculatedClipRect {
            rect: Rect {
                min: vec2(0., -0.5),
                max: vec2(0.5, 0.5),
            },
            world_to_clip_local: Affine2::IDENTITY,
        }]);
        let clipped = clip_polygon(Some(&clip), &quad(), Vec2::lerp);

        assert_eq!(clipped.len(), 4);
        assert!(clipped.iter().all(|(v, _)| 0. <= v.x && v.x <= 0.5));
        assert!(clipped.iter().all(|(v, _)| -0.5 <= v.y && v.y <= 0.5));
    }

    #[test]
    fn nested_clip_rects_compose() {
        let vertices = clip_polygon(
            Some(&calculated_clip([
                CalculatedClipRect {
                    rect: Rect {
                        min: vec2(-0.75, -0.75),
                        max: vec2(0.75, 0.75),
                    },
                    world_to_clip_local: Affine2::IDENTITY,
                },
                CalculatedClipRect {
                    rect: Rect {
                        min: vec2(-0.25, -1.),
                        max: vec2(0.25, 1.),
                    },
                    world_to_clip_local: Affine2::from_mat2(Mat2::from(Rot2::radians(0.3)))
                        .inverse(),
                },
            ])),
            &quad(),
            Vec2::lerp,
        );

        assert!(!vertices.is_empty());
        assert!(vertices.iter().all(|(v, _)| -0.75 <= v.x && v.x <= 0.75));
    }

    #[test]
    fn quad_outside_clip_rect_returns_empty_vertices_list() {
        assert!(clip_polygon(
            Some(&calculated_clip([CalculatedClipRect {
                rect: Rect {
                    min: vec2(2., 2.),
                    max: vec2(3., 3.),
                },
                world_to_clip_local: Affine2::IDENTITY,
            }])),
            &quad(),
            Vec2::lerp
        )
        .is_empty());
    }
}
