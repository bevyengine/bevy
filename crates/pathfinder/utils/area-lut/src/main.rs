// pathfinder/area-lut/src/main.rs

extern crate clap;
extern crate euclid;
extern crate image;

use clap::{App, Arg};
use euclid::default::Point2D;
use image::{ImageBuffer, Rgba};
use std::f32;
use std::path::Path;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

fn solve_line_y(p0: &Point2D<f32>, p1: &Point2D<f32>, y: f32) -> Point2D<f32> {
    let m = (p1.y - p0.y) / (p1.x - p0.x);
    Point2D::new(p0.x - (p0.y - y) / m, y)
}

fn area_tri(p0: Point2D<f32>, p1: Point2D<f32>) -> f32 {
    0.5 * (p1.x - p0.x) * (p0.y - p1.y)
}

fn area_rect(p0: Point2D<f32>, p1: Point2D<f32>) -> f32 {
    (p1.x - p0.x) * (p0.y - p1.y)
}

fn area(y: f32, dydx: f32) -> f32 {
    let (x_left, x_right) = (-0.5, 0.5);
    let (y_left, y_right) = (dydx * x_left + y, dydx * x_right + y);

    let (p0, p1) = (Point2D::new(x_left, y_left), Point2D::new(x_right, y_right));
    let p2 = solve_line_y(&p0, &p1, -0.5);
    let p3 = Point2D::new(p1.x, -0.5);
    let p4 = solve_line_y(&p0, &p1, 0.5);
    let p7 = Point2D::new(p1.x, 0.5);

    let alpha;
    if p0.y > 0.5 {
        if p1.y < -0.5 {
            // Case 0
            alpha = area_tri(p0, p1) - area_tri(p2, p1) - area_rect(p0, p7) + area_tri(p0, p4);
        } else if p1.y < 0.5 {
            // Case 6
            alpha = area_tri(p0, p1) - area_rect(p0, p7) + area_tri(p0, p4);
        } else {
            // Case 3
            alpha = 0.0;
        }
    } else if p0.y > -0.5 {
        if p1.y < -0.5 {
            // Case 1
            alpha = area_tri(p0, p1) - area_tri(p2, p1) - area_rect(p0, p7);
        } else {
            // Case 4
            alpha = area_tri(p0, p1) - area_rect(p0, p7);
        }
    } else {
        // Case 2
        alpha = -area_rect(p0, p7) + area_rect(p0, p3);
    }

    alpha
}

fn main() {
    let app = App::new("Pathfinder Area LUT Generator")
        .version("0.1")
        .author("The Pathfinder Project Developers")
        .about("Generates area lookup tables for use with Pathfinder")
        .arg(Arg::with_name("OUTPUT-PATH").help("The `.png` image to produce")
                                          .required(true)
                                          .index(1));

    let matches = app.get_matches();
    let image = ImageBuffer::from_fn(WIDTH, HEIGHT, |u, v| {
        if u == 0 {
            return Rgba([255, 255, 255, 255])
        }
        if u == WIDTH - 1 {
            return Rgba([0, 0, 0, 0])
        }

        let y = ((u as f32) - (WIDTH / 2) as f32) / 16.0;
        let dydx = -(v as f32) / 16.0;

        let alphas = [
            (area(y - 0.0, dydx) * 255.0).round() as u8,
            (area(y - 1.0, dydx) * 255.0).round() as u8,
            (area(y - 2.0, dydx) * 255.0).round() as u8,
            (area(y - 3.0, dydx) * 255.0).round() as u8,
        ];

        Rgba([alphas[0], alphas[1], alphas[2], alphas[3]])
    });

    let output_path = matches.value_of("OUTPUT-PATH").unwrap();
    let output_path = Path::new(output_path);

    image.save(&output_path).unwrap();
}
