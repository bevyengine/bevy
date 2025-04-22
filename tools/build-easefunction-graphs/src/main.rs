//! Generates graphs for the `EaseFunction` docs.

#![expect(clippy::print_stdout, reason = "Allowed in tools.")]

use std::path::PathBuf;

use bevy_math::curve::{CurveExt, EaseFunction, EasingCurve, JumpAt};
use svg::{
    node::element::{self, path::Data},
    Document,
};

fn main() {
    let root_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("Please run via cargo or set CARGO_MANIFEST_DIR"),
    );
    let directory = root_dir
        .join("../../crates/bevy_math/images/easefunction")
        .canonicalize()
        .unwrap();

    for function in [
        EaseFunction::SineIn,
        EaseFunction::SineOut,
        EaseFunction::SineInOut,
        EaseFunction::QuadraticIn,
        EaseFunction::QuadraticOut,
        EaseFunction::QuadraticInOut,
        EaseFunction::CubicIn,
        EaseFunction::CubicOut,
        EaseFunction::CubicInOut,
        EaseFunction::QuarticIn,
        EaseFunction::QuarticOut,
        EaseFunction::QuarticInOut,
        EaseFunction::QuinticIn,
        EaseFunction::QuinticOut,
        EaseFunction::QuinticInOut,
        EaseFunction::SmoothStepIn,
        EaseFunction::SmoothStepOut,
        EaseFunction::SmoothStep,
        EaseFunction::SmootherStepIn,
        EaseFunction::SmootherStepOut,
        EaseFunction::SmootherStep,
        EaseFunction::CircularIn,
        EaseFunction::CircularOut,
        EaseFunction::CircularInOut,
        EaseFunction::ExponentialIn,
        EaseFunction::ExponentialOut,
        EaseFunction::ExponentialInOut,
        EaseFunction::ElasticIn,
        EaseFunction::ElasticOut,
        EaseFunction::ElasticInOut,
        EaseFunction::BackIn,
        EaseFunction::BackOut,
        EaseFunction::BackInOut,
        EaseFunction::BounceIn,
        EaseFunction::BounceOut,
        EaseFunction::BounceInOut,
        EaseFunction::Linear,
        EaseFunction::Steps(4, JumpAt::Start),
        EaseFunction::Steps(4, JumpAt::End),
        EaseFunction::Steps(4, JumpAt::None),
        EaseFunction::Steps(4, JumpAt::Both),
        EaseFunction::Elastic(50.0),
    ] {
        let curve = EasingCurve::new(0.0, 1.0, function);
        let samples = curve
            .map(|y| {
                // Fit into svg coordinate system
                1. - y
            })
            .graph()
            .samples(100)
            .unwrap()
            .collect::<Vec<_>>();

        // Curve can go out past endpoints
        let mut min = 0.0f32;
        let mut max = 1.0f32;
        for &(_, y) in &samples {
            min = min.min(y);
            max = max.max(y);
        }

        let graph = element::Polyline::new()
            .set("points", samples)
            .set("fill", "none")
            .set("stroke", "red")
            .set("stroke-width", 0.04);

        let guides = element::Path::new()
            .set("fill", "none")
            .set("stroke", "var(--main-color)")
            .set("stroke-width", 0.02)
            .set("d", {
                // Interval
                let mut data = Data::new()
                    .move_to((0, 0))
                    .line_to((0, 1))
                    .move_to((1, 0))
                    .line_to((1, 1));
                // Dotted lines y=0 | y=1
                for y in 0..=1 {
                    data = data.move_to((0, y));
                    for _ in 0..5 {
                        data = data.move_by((0.1, 0.)).line_by((0.1, 0.));
                    }
                }
                data
            });

        let opt_tag = match function {
            EaseFunction::Steps(_n, jump_at) => format!("{jump_at:?}"),
            _ => String::new(),
        };

        let name = format!("{opt_tag}{function:?}");
        let tooltip = element::Title::new(&name);

        const MARGIN: f32 = 0.04;
        let document = Document::new()
            .set("width", "6em")
            .set(
                "viewBox",
                (
                    -MARGIN,
                    min - MARGIN,
                    1. + 2. * MARGIN,
                    max - min + 2. * MARGIN,
                ),
            )
            .add(tooltip)
            .add(guides)
            .add(graph);

        let file_path = directory
            .join(name.split('(').next().unwrap())
            .with_extension("svg");
        println!("saving {file_path:?}");
        svg::save(file_path, &document).unwrap();
    }
}
